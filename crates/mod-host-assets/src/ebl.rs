use std::ptr::NonNull;

use from_singleton::FromSingleton;

use crate::{
    alloc::DlStdAllocator,
    rtti::{find_vmt, FindError},
};

#[repr(C)]
pub struct EblFileManager {
    _vtable: usize,
    utility: *mut EblUtility,
}

#[repr(C)]
pub struct EblUtility {
    vtable: NonNull<EblUtilityVtable>,
}

#[repr(C)]
pub struct EblUtilityVtable {
    _dtor: usize,
    pub make_ebl_object: MakeEblObject,
    _delete_ebl_object: usize,
    pub mount_ebl: MountEbl,
}

type MakeEblObject = extern "C" fn(
    this: NonNull<EblUtility>,
    path: *const u16,
    allocator: DlStdAllocator,
) -> Option<NonNull<u8>>;

type MountEbl = extern "C" fn(
    this: NonNull<EblUtility>,
    mount_name: *const u16,
    header_path: *const u16,
    data_path: *const u16,
    allocator: DlStdAllocator,
    rsa_key: *const u8,
    key_len: usize,
) -> bool;

impl EblFileManager {
    /// # Safety
    /// Same as [`find_vmt`].
    pub unsafe fn ebl_utility_vtable(
        image_base: *const u8,
    ) -> Result<NonNull<EblUtilityVtable>, FindError> {
        if let Some(vtable) = Self::ebl_utility_vtable_from_singleton() {
            return Ok(vtable);
        }

        find_vmt(image_base, "DLEncryptedBinderLightUtility")
    }

    fn ebl_utility_vtable_from_singleton() -> Option<NonNull<EblUtilityVtable>> {
        let ptr = from_singleton::address_of::<Self>()?;
        unsafe { Some(ptr.as_ref().utility.as_ref()?.vtable) }
    }
}

impl FromSingleton for EblFileManager {
    fn name() -> std::borrow::Cow<'static, str> {
        "CSEblFileManager".into()
    }
}
