use std::{mem, ptr::NonNull};

use from_singleton::FromSingleton;
use me3_binary_analysis::{pe, rtti::ClassMap};
use pelite::pe::Pe;
use regex::bytes::Regex;
use windows::core::{PCSTR, PCWSTR};

use crate::{alloc::DlStdAllocator, bhd5::Bhd5Header};

#[repr(C)]
pub struct EblFileManager;

#[repr(C)]
pub struct EblFileDevice {
    _vtable: usize,
    _unk00: [u8; 0xa8],
    bhd_header: Option<NonNull<Bhd5Header>>,
    bucket_count: u32,
    buckets: Option<NonNull<u32>>,
}

#[repr(C)]
struct EblUtilityVtable {
    _dtor: usize,
    make_ebl_object: MakeEblObject,
}

pub type MakeEblObject = unsafe extern "C" fn(
    this: usize,
    path: PCWSTR,
    allocator: DlStdAllocator,
) -> Option<NonNull<()>>;

/// # Note
///
/// The key_len parameter may be inaccurate, e.g. it's off by one for sd.bhd in ER.
pub type MountEbl = unsafe extern "C" fn(
    mount_name: PCWSTR,
    bhd_path: PCWSTR,
    bdt_path: PCWSTR,
    allocator: DlStdAllocator,
    rsa_key: PCSTR,
    key_len: usize,
) -> bool;

impl EblFileManager {
    pub fn make_ebl_object<'a, P>(program: P, class_map: &ClassMap) -> Option<MakeEblObject>
    where
        P: Pe<'a>,
    {
        class_map
            .get("DLEBL::DLEncryptedBinderLightUtility")
            .and_then(|vmt| unsafe { Some(vmt.first()?.as_ref::<EblUtilityVtable>()) })
            .or_else(|| Self::make_ebl_object_from_singleton(program))
            .map(|vmt| vmt.make_ebl_object)
    }

    fn make_ebl_object_from_singleton<'a, P>(program: P) -> Option<&'a EblUtilityVtable>
    where
        P: Pe<'a>,
    {
        let ptr = from_singleton::address_of::<Self>()?.cast::<*const u8>();

        let rdata = pe::section(program, ".rdata")
            .ok()
            .and_then(|s| program.get_section_bytes(s).ok())?;

        // Depending on Dantelion2 version, there may be a vtable for the CSEblFileManager
        // instance before the EblUtilityVtable pointer.
        unsafe {
            let ptr = if rdata.as_ptr_range().contains(ptr.as_ref()) {
                ptr.add(1).read()
            } else {
                ptr.read()
            };

            ptr.cast::<*const EblUtilityVtable>()
                .as_ref()
                .and_then(|ptr| ptr.as_ref())
        }
    }
}

impl EblFileDevice {
    pub fn bhd_header(&self) -> Option<&Bhd5Header> {
        self.bhd_header.map(|ptr| unsafe { ptr.as_ref() })
    }

    /// # Safety
    ///
    /// The buffer pointed to by `contents` should be aligned and allocated
    /// with a [`DlStdAllocator`], so that it may be freed later.
    pub unsafe fn assign_bhd_contents(&mut self, contents: *mut Bhd5Header) {
        self.bhd_header = NonNull::new(contents);

        if let Some(contents) = &self.bhd_header {
            let header = unsafe { contents.as_ref() };

            let buckets = header.buckets();

            self.bucket_count = buckets.len() as u32;
            self.buckets = Some(buckets.cast());
        }
    }
}

pub fn mount_ebl<'a, P>(program: P) -> Option<MountEbl>
where
    P: Pe<'a>,
{
    let text = pe::section(program, ".text")
        .ok()
        .and_then(|s| program.get_section_bytes(s).ok())?;

    let mount_re = Regex::new(
        r"(?s-u)\x48\x8b\x45.\x48\x89\x44\x24\x28[\x48|\x4c]\x89[\x44\x4c\x54\x5c\x64\x6c\x74\x7c]\x24\x20\x4c\x8b\x0d.{4}[\x48|\x49]\x8b[\xd0-\xd7][\x48|\x49]\x8b[\xc8-\xcf]\xe8(.{4})\x0f\xb6\xd8(?:(?:\x48\x83\x7d.\x08)|(?:\x48\x83\x7c\x24.\x08))\x72."
    )
    .unwrap();

    let (_, [call_disp32]) = mount_re.captures(text)?.extract();

    let call_bytes = <[u8; 4]>::try_from(call_disp32).unwrap();

    let mount_ebl = unsafe {
        mem::transmute(
            call_disp32
                .as_ptr_range()
                .end
                .offset(i32::from_le_bytes(call_bytes) as _),
        )
    };

    Some(mount_ebl)
}

impl FromSingleton for EblFileManager {
    fn name() -> std::borrow::Cow<'static, str> {
        "CSEblFileManager".into()
    }
}
