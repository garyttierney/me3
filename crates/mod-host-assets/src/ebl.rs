use std::ptr::NonNull;

use from_singleton::FromSingleton;
use me3_binary_analysis::{pe, rtti::ClassMap};
use pelite::pe::Pe;

use crate::alloc::DlStdAllocator;

#[repr(C)]
pub struct EblFileManager;

#[repr(C)]
struct EblUtilityVtable {
    _dtor: usize,
    make_ebl_object: MakeEblObject,
}

type MakeEblObject = unsafe extern "C" fn(
    this: usize,
    path: *const u16,
    allocator: DlStdAllocator,
) -> Option<NonNull<u8>>;

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

impl FromSingleton for EblFileManager {
    fn name() -> std::borrow::Cow<'static, str> {
        "CSEblFileManager".into()
    }
}
