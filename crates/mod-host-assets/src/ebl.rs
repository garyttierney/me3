use std::ptr::NonNull;

use from_singleton::FromSingleton;

use crate::{
    alloc::DlStdAllocator,
    pe,
    rtti::{find_vmt, FindError},
};

#[repr(C)]
pub struct EblFileManager;

pub struct EblUtilityVtable {
    pub make_ebl_object: MakeEblObject,
    pub mount_ebl: MountEbl,
}

#[repr(C)]
struct EblUtilityVtableER {
    _dtor: usize,
    make_ebl_object: MakeEblObject,
    _delete_ebl_object: usize,
    mount_ebl: MountEbl,
}

#[repr(C)]
struct EblUtilityVtableNR {
    _dtor: usize,
    make_ebl_object: MakeEblObject,
    _make_ebl_object_unk_str: usize,
    _delete_ebl_object: usize,
    mount_ebl: MountEbl,
}

type MakeEblObject =
    extern "C" fn(this: usize, path: *const u16, allocator: DlStdAllocator) -> Option<NonNull<u8>>;

type MountEbl = extern "C" fn(
    this: usize,
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
    pub unsafe fn ebl_utility_vtable(image_base: *const u8) -> Result<EblUtilityVtable, FindError> {
        // SAFETY: Upheld by caller.
        unsafe {
            if let Some(vtable) = Self::ebl_utility_vtable_from_singleton(image_base) {
                return Ok(vtable);
            }

            find_vmt(image_base, "DLEncryptedBinderLightUtility").map(|ptr| {
                let &EblUtilityVtableER {
                    make_ebl_object,
                    mount_ebl,
                    ..
                } = ptr.as_ref();

                EblUtilityVtable {
                    make_ebl_object,
                    mount_ebl,
                }
            })
        }
    }

    unsafe fn ebl_utility_vtable_from_singleton(image_base: *const u8) -> Option<EblUtilityVtable> {
        unsafe {
            let ptr = from_singleton::address_of::<Self>()?.cast::<*const u8>();

            let [rdata] = pe::sections(image_base, [".rdata"]).ok()?;

            // Depending on Dantelion2 version, there may be a vtable for the CSEblFileManager
            // instance before the EblUtilityVtable pointer.
            let (make_ebl_object, mount_ebl) = if rdata.as_ptr_range().contains(ptr.as_ref()) {
                let &EblUtilityVtableER {
                    make_ebl_object,
                    mount_ebl,
                    ..
                } = ptr
                    .add(1)
                    .read()
                    .cast::<*const EblUtilityVtableER>()
                    .as_ref()?
                    .as_ref()?;

                (make_ebl_object, mount_ebl)
            } else {
                let &EblUtilityVtableNR {
                    make_ebl_object,
                    mount_ebl,
                    ..
                } = ptr
                    .read()
                    .cast::<*const EblUtilityVtableNR>()
                    .as_ref()?
                    .as_ref()?;

                (make_ebl_object, mount_ebl)
            };

            Some(EblUtilityVtable {
                make_ebl_object,
                mount_ebl,
            })
        }
    }
}

impl FromSingleton for EblFileManager {
    fn name() -> std::borrow::Cow<'static, str> {
        "CSEblFileManager".into()
    }
}
