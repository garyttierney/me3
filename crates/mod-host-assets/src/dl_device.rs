use std::{
    borrow::Cow,
    collections::VecDeque,
    fmt,
    ops::Range,
    ptr::{self, NonNull},
};

use me3_binary_analysis::pe;
use me3_mod_host_types::{
    alloc::DlStdAllocator,
    string::{DlUtf16String, EncodingError},
    vector::DlVector,
};
use pelite::pe::Pe;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rdvec::Vec as DynVec;
use thiserror::Error;
use windows::{
    core::PCWSTR,
    Win32::System::Threading::{EnterCriticalSection, LeaveCriticalSection, CRITICAL_SECTION},
};

#[repr(C)]
pub struct DlDeviceManager {
    devices: DlVector<NonNull<DlDevice>>,
    spis: DlVector<NonNull<*mut ()>>,
    disk_device: NonNull<DlDevice>,
    virtual_roots: DlVector<DlVirtualRoot>,
    bnd3_mounts: DlVector<DlVirtualMount>,
    bnd4_mounts: DlVector<DlVirtualMount>,
    bnd3_spi: NonNull<*mut ()>,
    bnd4_spi: NonNull<*mut ()>,
    mutex_vtable: NonNull<*mut ()>,
    critical_section: CRITICAL_SECTION,
    _unke8: bool,
}

#[repr(C)]
pub struct DlDevice {
    pub(crate) vtable: NonNull<DlDeviceVtable>,
}

pub type DlFileOperator = NonNull<DlFileOperatorVtable>;

#[repr(C)]
pub struct DlVirtualRoot {
    root: DlUtf16String,
    expanded: DlUtf16String,
}

#[repr(C)]
#[derive(Clone)]
pub struct DlVirtualMount {
    root: DlUtf16String,
    device: NonNull<DlDevice>,
    size: usize,
}

pub struct DlDeviceManagerGuard {
    inner: NonNull<DlDeviceManager>,
}

type DlDeviceOpen = unsafe extern "C" fn(
    NonNull<DlDevice>,
    path: NonNull<DlUtf16String>,
    path_cstr: PCWSTR,
    NonNull<()>,
    DlStdAllocator,
    bool,
) -> Option<NonNull<DlFileOperator>>;

#[repr(C)]
pub struct DlDeviceVtable {
    _dtor: usize,
    open_file: DlDeviceOpen,
}

type DlFileOperatorSetPath =
    unsafe extern "C" fn(NonNull<DlFileOperator>, path: NonNull<DlUtf16String>, bool, bool) -> bool;

#[repr(C)]
pub struct DlFileOperatorVtable {
    _dtor: usize,
    _copy: usize,
    pub set_path: DlFileOperatorSetPath,
    pub set_path2: DlFileOperatorSetPath,
    pub set_path3: DlFileOperatorSetPath,
}

pub struct BndSnapshot {
    inner: Vec<Vec<u16>>,
}

#[derive(Debug, Default)]
pub struct VfsMounts {
    inner: Vec<DlVirtualMount>,
}

pub struct VfsPushGuard<'a> {
    owner: &'a mut DlDeviceManagerGuard,
    old_devices_len: usize,
    old_mounts_len: usize,
}

pub fn find_device_manager<'a, P>(program: P) -> Result<NonNull<DlDeviceManager>, FindError>
where
    P: Pe<'a>,
{
    let [data, rdata] = pe::sections(program, [".data", ".rdata"]).map_err(FindError::Section)?;

    let data = program.get_section_bytes(data)?;
    let rdata = program.get_section_bytes(rdata)?;

    let (_, data_ptrs, _) = unsafe { data.align_to::<usize>() };

    let manager_ptr = data_ptrs.par_iter().find_first(move |ptr| unsafe {
        let manager_ptr = **ptr as *const DlDeviceManager;

        let data_range = data.as_ptr_range();

        if !data_range.contains(&manager_ptr.cast())
            || !data_range.contains(&manager_ptr.add(1).byte_sub(1).cast())
        {
            return false;
        }

        let rdata_range = rdata.as_ptr_range();

        verify_dl_device_manager_layout(manager_ptr, data_range, rdata_range)
    });

    manager_ptr
        .and_then(|ptr| NonNull::new(*ptr as *mut DlDeviceManager))
        .ok_or(FindError::Instance)
}

/// # Safety
///
/// `ptr` must be in bounds for all reads.
unsafe fn verify_dl_device_manager_layout(
    device_manager: *const DlDeviceManager,
    data_range: Range<*const u8>,
    rdata_range: Range<*const u8>,
) -> bool {
    if !device_manager.is_aligned() {
        return false;
    }

    // SAFETY: pointer is aligned for all reads, in bounds by precondition.
    unsafe {
        let Some((_, _, _, alloc)) =
            DlVector::try_read_raw_parts(&raw const (*device_manager).devices)
        else {
            return false;
        };

        if !data_range.contains(&(alloc as *const u8)) {
            return false;
        }

        if !matches!(
            DlVector::try_read_raw_parts(&raw const (*device_manager).spis),
            Some((_, _, _, other_alloc)) if other_alloc == alloc
        ) {
            return false;
        }

        if !matches!(
            DlVector::try_read_raw_parts(&raw const (*device_manager).virtual_roots),
            Some((_, _, _, other_alloc)) if other_alloc == alloc
        ) {
            return false;
        }

        if !matches!(
            DlVector::try_read_raw_parts(&raw const (*device_manager).bnd3_mounts),
            Some((_, _, _, other_alloc)) if other_alloc == alloc
        ) {
            return false;
        }

        if !matches!(
            DlVector::try_read_raw_parts(&raw const (*device_manager).bnd4_mounts),
            Some((_, _, _, other_alloc)) if other_alloc == alloc
        ) {
            return false;
        }

        let disk_device = ptr::read(
            &raw const (*device_manager).disk_device as *const *mut NonNull<DlDeviceVtable>,
        );

        if !disk_device.is_aligned() || disk_device.is_null() {
            return false;
        }

        let bnd3_spi = ptr::read(&raw const (*device_manager).bnd3_spi as *const *mut *mut ());

        if !bnd3_spi.is_aligned() || bnd3_spi.is_null() {
            return false;
        }

        let bnd4_spi = ptr::read(&raw const (*device_manager).bnd4_spi as *const *mut *mut ());

        if !bnd4_spi.is_aligned() || bnd4_spi.is_null() {
            return false;
        }

        let mutex_vtable =
            ptr::read(&raw const (*device_manager).mutex_vtable as *const *mut *mut ());

        if !mutex_vtable.is_aligned() || !rdata_range.contains(&(mutex_vtable as *const u8)) {
            return false;
        }
    }

    true
}

impl DlDeviceManager {
    pub fn lock(ptr: NonNull<DlDeviceManager>) -> DlDeviceManagerGuard {
        unsafe {
            EnterCriticalSection(&raw mut (*ptr.as_ptr()).critical_section);
        }

        DlDeviceManagerGuard { inner: ptr }
    }
}

impl DlDeviceManagerGuard {
    pub fn snapshot(&self) -> Result<BndSnapshot, EncodingError> {
        let device_manager = unsafe { self.inner.as_ref() };

        let snapshot = device_manager
            .bnd4_mounts
            .iter()
            .map(|m| m.root.get().map(|s| s.as_slice().to_owned()))
            .collect::<Result<Vec<Vec<u16>>, EncodingError>>()?;

        Ok(BndSnapshot::new(snapshot))
    }

    pub fn extract_new(&mut self, snapshot: BndSnapshot) -> VfsMounts {
        let device_manager = unsafe { self.inner.as_mut() };

        let mut removed_mounts = VecDeque::new();

        for i in (0..device_manager.bnd4_mounts.len()).rev() {
            if !snapshot.has_mount(&device_manager.bnd4_mounts[i]) {
                removed_mounts.push_front(device_manager.bnd4_mounts.remove(i));
            }
        }

        for i in (0..device_manager.devices.len()).rev() {
            let device = device_manager.devices[i];
            if removed_mounts.iter().any(|m| m.device == device) {
                device_manager.devices.remove(i);
            }
        }

        VfsMounts {
            inner: removed_mounts.into(),
        }
    }

    pub fn push_vfs_mounts(&mut self, vfs: &VfsMounts) -> VfsPushGuard<'_> {
        let device_manager = unsafe { self.inner.as_mut() };

        let old_devices_len = device_manager.devices.len();

        device_manager
            .devices
            .extend(&mut vfs.inner.iter().map(|m| m.device));

        let old_mounts_len = device_manager.bnd4_mounts.len();

        device_manager
            .bnd4_mounts
            .extend(&mut vfs.inner.iter().cloned());

        VfsPushGuard {
            owner: self,
            old_devices_len,
            old_mounts_len,
        }
    }

    pub fn expand_path<'a>(&self, path: &'a [u16]) -> Cow<'a, [u16]> {
        let device_manager = unsafe { self.inner.as_ref() };

        let mut expanded = Cow::Borrowed(path);

        loop {
            let Some(root_end) = expanded.windows(2).position(is_root_separator) else {
                break;
            };

            let root = &expanded[..root_end];

            let virtual_root = device_manager
                .virtual_roots
                .iter()
                .find(|v| v.root.get().is_ok_and(|r| root == r.as_slice()));

            if let Some(replace_with) = virtual_root.and_then(|v| v.expanded.get().ok()) {
                let mut new = replace_with.as_slice().to_owned();
                new.extend_from_slice(&expanded[root_end + 2..]);
                expanded = Cow::Owned(new);
            } else {
                break;
            }
        }

        expanded
    }

    pub fn open_disk_file(&self) -> DlDeviceOpen {
        unsafe {
            let device_manager = self.inner.as_ref();
            device_manager
                .disk_device
                .as_ref()
                .vtable
                .as_ref()
                .open_file
        }
    }
}

impl BndSnapshot {
    fn new(vec: Vec<Vec<u16>>) -> Self {
        let mut sorted = vec;
        sorted.sort_unstable();
        Self { inner: sorted }
    }

    fn has_mount(&self, mount: &DlVirtualMount) -> bool {
        mount.root.get().is_ok_and(|r| {
            self.inner
                .binary_search_by(|v| Ord::cmp(&**v, r.as_slice()))
                .is_ok()
        })
    }
}

impl VfsMounts {
    pub const fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn open_disk_file_fn(&self) -> Option<DlDeviceOpen> {
        unsafe {
            let ptr = self.inner.first()?.device;
            Some(ptr::read(&raw const ptr.read().vtable.as_ref().open_file))
        }
    }

    pub fn append(&mut self, new: VfsMounts) {
        let mut inner = new.inner;
        self.inner.append(&mut inner);
    }

    /// # Safety
    /// only if passed arguments from `DlDeviceOpen`.
    pub unsafe fn try_open_file(
        &self,
        path: NonNull<DlUtf16String>,
        path_cstr: PCWSTR,
        container: NonNull<()>,
        allocator: DlStdAllocator,
        is_temp_file: bool,
    ) -> Option<NonNull<DlFileOperator>> {
        let path_bytes = unsafe { path.as_ref().get().ok()?.as_slice() };

        let root_end = path_bytes.windows(2).position(is_root_separator)?;
        let root = &path_bytes[..root_end];

        self.inner
            .iter()
            .find(|m| m.root.get().is_ok_and(|r| root == r.as_slice()))
            .and_then(|m| {
                let f = unsafe { ptr::read(&raw const m.device.read().vtable.as_ref().open_file) };
                unsafe {
                    f(
                        m.device,
                        path,
                        path_cstr,
                        container,
                        allocator,
                        is_temp_file,
                    )
                }
            })
    }

    pub fn devices(&self) -> impl Iterator<Item = NonNull<DlDevice>> {
        self.inner.iter().map(|m| m.device)
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

fn is_root_separator(w: &[u16]) -> bool {
    w[0] == ':' as u16 && w[1] == '/' as u16
}

#[derive(Clone, Debug, Error)]
pub enum FindError {
    #[error(transparent)]
    Pe(#[from] pelite::Error),
    #[error("PE section \"{0}\" is missing")]
    Section(&'static str),
    #[error("DlDeviceManager instance not found")]
    Instance,
}

impl Drop for DlDeviceManagerGuard {
    fn drop(&mut self) {
        unsafe {
            LeaveCriticalSection(&mut self.inner.as_mut().critical_section);
        }
    }
}

impl Drop for VfsPushGuard<'_> {
    fn drop(&mut self) {
        unsafe {
            self.owner
                .inner
                .as_mut()
                .devices
                .truncate(self.old_devices_len);

            self.owner
                .inner
                .as_mut()
                .bnd4_mounts
                .truncate(self.old_mounts_len);
        }
    }
}

impl fmt::Debug for DlVirtualMount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DlVirtualMount")
            .field("root", &self.root.get().map(|r| r.to_string()))
            .field("device", &self.device)
            .finish()
    }
}

unsafe impl Send for VfsMounts {}

unsafe impl Sync for VfsMounts {}
