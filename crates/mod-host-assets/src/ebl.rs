use std::ptr::NonNull;

use from_singleton::FromSingleton;

use crate::{
    alloc::DlStdAllocator,
    pe,
    rtti::{find_vmt, FindError},
};

#[repr(C)]
pub struct EblFileManager;

#[derive(Debug)]
pub struct EblUtilityVtable {
    pub make_ebl_object: MakeEblObject,
}

#[repr(C)]
struct EblUtilityVtableER {
    _dtor: usize,
    make_ebl_object: MakeEblObject,
}

#[repr(C)]
struct EblUtilityVtableNR {
    _dtor: usize,
    make_ebl_object: MakeEblObject,
}

type MakeEblObject = unsafe extern "C" fn(
    this: usize,
    path: *const u16,
    allocator: DlStdAllocator,
) -> Option<NonNull<u8>>;

impl EblFileManager {
    /// # Safety
    /// Same as [`find_vmt`].
    pub unsafe fn make_ebl_object(image_base: *const u8) -> Result<MakeEblObject, FindError> {
        // SAFETY: Upheld by caller.
        unsafe {
            if let Some(make_ebl_object) = Self::make_ebl_object_from_singleton(image_base) {
                return Ok(make_ebl_object);
            }

            find_vmt(image_base, "DLEncryptedBinderLightUtility").map(|ptr| {
                let &EblUtilityVtableER {
                    make_ebl_object, ..
                } = ptr.as_ref();

                make_ebl_object
            })
        }
    }

    unsafe fn make_ebl_object_from_singleton(image_base: *const u8) -> Option<MakeEblObject> {
        unsafe {
            let ptr = from_singleton::address_of::<Self>()?.cast::<*const u8>();

            let [rdata] = pe::sections(image_base, [".rdata"]).ok()?;

            // Depending on Dantelion2 version, there may be a vtable for the CSEblFileManager
            // instance before the EblUtilityVtable pointer.
            if rdata.as_ptr_range().contains(ptr.as_ref()) {
                let &EblUtilityVtableER {
                    make_ebl_object, ..
                } = ptr
                    .add(1)
                    .read()
                    .cast::<*const EblUtilityVtableER>()
                    .as_ref()?
                    .as_ref()?;

                Some(make_ebl_object)
            } else {
                let &EblUtilityVtableNR {
                    make_ebl_object, ..
                } = ptr
                    .read()
                    .cast::<*const EblUtilityVtableNR>()
                    .as_ref()?
                    .as_ref()?;

                Some(make_ebl_object)
            }
        }
    }
}

impl FromSingleton for EblFileManager {
    fn name() -> std::borrow::Cow<'static, str> {
        "CSEblFileManager".into()
    }
}
