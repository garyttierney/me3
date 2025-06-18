use std::{
    borrow::Cow,
    collections::VecDeque,
    fmt, mem,
    ops::Range,
    ptr::{self, NonNull},
};

use cxx_stl::vec::CxxVec;
use thiserror::Error;
use windows::Win32::System::Threading::{
    EnterCriticalSection, LeaveCriticalSection, CRITICAL_SECTION,
};

use crate::{
    alloc::DlStdAllocator,
    pe,
    string::{DlUtf16String, EncodingError},
};

#[repr(C)]
pub struct DlDeviceManager {
    devices: CxxVec<NonNull<DlDevice>, DlStdAllocator>,
    spis: CxxVec<NonNull<u8>, DlStdAllocator>,
    disk_device: NonNull<DlDevice>,
    virtual_roots: CxxVec<DlVirtualRoot, DlStdAllocator>,
    bnd3_mounts: CxxVec<DlVirtualMount, DlStdAllocator>,
    bnd4_mounts: CxxVec<DlVirtualMount, DlStdAllocator>,
    bnd3_spi: NonNull<u8>,
    bnd4_spi: NonNull<u8>,
    mutex_vtable: usize,
    critical_section: CRITICAL_SECTION,
    _unke8: bool,
}

pub type DlDevice = NonNull<DlDeviceVtable>;

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
    path_cstr: *const u16,
    NonNull<u8>,
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
    old_len: usize,
}

/// # Safety
/// [`pelite::pe64::PeView::module`] must be safe to call on `image_base`
pub unsafe fn find_device_manager(
    image_base: *const u8,
) -> Result<NonNull<DlDeviceManager>, FindError> {
    // SAFETY: must be upheld by caller.
    let [data, rdata] = unsafe { pe::sections(image_base, [".data", ".rdata"])? };

    const SIZE: usize = mem::size_of::<*const u8>();
    const ALIGNMENT: usize = mem::align_of::<*const u8>();

    let data_range = data.as_ptr_range();

    let Range {
        start,
        end: data_end,
    } = data_range;

    let mut data_ptr =
        start.wrapping_byte_offset(start.align_offset(ALIGNMENT) as isize - SIZE as isize);

    while data_ptr < data_end {
        data_ptr = data_ptr.wrapping_byte_add(SIZE);

        // SAFETY: pointer is aligned and non-null.
        let manager_ptr = unsafe { data_ptr.cast::<*const DlDeviceManager>().read() };

        if !data_range.contains(&manager_ptr.add(1).cast::<u8>().sub(1))
            || !data_range.contains(&manager_ptr.cast())
        {
            continue;
        }

        // SAFETY: pointer is in bounds of ".data".
        if verify_dl_device_manager_layout(manager_ptr, data_range.clone(), rdata.as_ptr_range()) {
            return Ok(NonNull::new(manager_ptr as _).unwrap());
        }
    }

    Err(FindError::Instance)
}

/// # Safety
/// `ptr` must be in bounds for all reads.
fn verify_dl_device_manager_layout(
    device_manager: *const DlDeviceManager,
    data_range: Range<*const u8>,
    rdata_range: Range<*const u8>,
) -> bool {
    if !device_manager.is_aligned() {
        return false;
    }

    let ptr = device_manager.cast::<*const usize>();

    macro_rules! verify_vec {
        ($v:expr, $alloc:expr) => {
            #[allow(unused_unsafe)]
            unsafe {
                if $alloc != $v.read() {
                    return false;
                }

                let first = $v.add(1).read();
                let last = $v.add(2).read();
                let end = $v.add(3).read();

                if !first.is_aligned() || !last.is_aligned() || !end.is_aligned() {
                    return false;
                }

                if first > last || last > end {
                    return false;
                }
            }
        };
    }

    // SAFETY: pointer is aligned for all reads, in bounds by precondition.
    unsafe {
        let alloc = ptr.read();

        if !alloc.is_aligned() || !data_range.contains(&alloc.cast()) {
            return false;
        }

        verify_vec!(ptr, alloc);

        verify_vec!(
            &raw const (*device_manager).spis as *const *const usize,
            alloc
        );

        verify_vec!(
            &raw const (*device_manager).virtual_roots as *const *const usize,
            alloc
        );

        verify_vec!(
            &raw const (*device_manager).bnd3_mounts as *const *const usize,
            alloc
        );

        verify_vec!(
            &raw const (*device_manager).bnd4_mounts as *const *const usize,
            alloc
        );

        let disk_device =
            ptr::read(&raw const (*device_manager).disk_device as *const *const usize);

        if disk_device.is_null() || !disk_device.is_aligned() {
            return false;
        }

        let bnd3_spi = ptr::read(&raw const (*device_manager).bnd3_spi as *const *const usize);

        if bnd3_spi.is_null() || !bnd3_spi.is_aligned() {
            return false;
        }

        let bnd4_spi = ptr::read(&raw const (*device_manager).bnd4_spi as *const *const usize);

        if bnd4_spi.is_null() || !bnd4_spi.is_aligned() {
            return false;
        }

        let mutex_vtable =
            ptr::read(&raw const (*device_manager).mutex_vtable as *const *const usize);

        if !mutex_vtable.is_aligned() || !rdata_range.contains(&mutex_vtable.cast()) {
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
            .map(|m| m.root.get().map(|s| s.as_bytes().to_owned()))
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

        VfsMounts {
            inner: removed_mounts.into(),
        }
    }

    pub fn push_vfs(&mut self, vfs: &VfsMounts) -> VfsPushGuard<'_> {
        let device_manager = unsafe { self.inner.as_mut() };

        let old_len = device_manager.bnd4_mounts.len();

        device_manager.bnd4_mounts.extend(vfs.inner.iter().cloned());

        VfsPushGuard {
            owner: self,
            old_len,
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
                .find(|v| v.root.get().is_ok_and(|r| root == r.as_bytes()));

            if let Some(replace_with) = virtual_root.and_then(|v| v.expanded.get().ok()) {
                let mut new = replace_with.as_bytes().to_owned();
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
            device_manager.disk_device.as_ref().as_ref().open_file
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
                .binary_search_by(|v| Ord::cmp(&**v, r.as_bytes()))
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
            Some(ptr::read(&raw const ptr.read().as_ref().open_file))
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
        path_cstr: *const u16,
        container: NonNull<u8>,
        allocator: DlStdAllocator,
        is_temp_file: bool,
    ) -> Option<NonNull<DlFileOperator>> {
        let path_bytes = unsafe { path.as_ref().get().ok()?.as_bytes() };

        let root_end = path_bytes.windows(2).position(is_root_separator)?;
        let root = &path_bytes[..root_end];

        self.inner
            .iter()
            .find(|m| m.root.get().is_ok_and(|r| root == r.as_bytes()))
            .and_then(|m| {
                let f = unsafe { ptr::read(&raw const m.device.read().as_ref().open_file) };
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

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

fn is_root_separator(w: &[u16]) -> bool {
    w[0] == ':' as u16 && w[1] == '/' as u16
}

#[derive(Clone, Debug, Error)]
pub enum FindError {
    #[error("{0}")]
    PeSection(pe::SectionError),
    #[error("DlDeviceManager instance not found")]
    Instance,
}

impl From<pe::SectionError> for FindError {
    fn from(value: pe::SectionError) -> Self {
        FindError::PeSection(value)
    }
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
            self.owner.inner.as_mut().bnd4_mounts.truncate(self.old_len);
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
