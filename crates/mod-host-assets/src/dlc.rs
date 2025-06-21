use std::ptr::{self, NonNull};

use crate::rtti::{find_vmt, FindError};

type MountDlcEbl = unsafe extern "C" fn(usize, bool, usize, usize);

/// # Safety
/// Same as [`find_vmt`].
pub unsafe fn mount_dlc_ebl(image_base: *const u8) -> Result<MountDlcEbl, FindError> {
    #[repr(C)]
    struct Vtable {
        other: [usize; 8],
        mount_dlc_ebl: MountDlcEbl,
    }

    // SAFETY: upheld by caller.
    let vtable: NonNull<Vtable> = unsafe { find_vmt(image_base, "CS::CSDlcPlatformImp_forSteam")? };

    // SAFETY: pointer returned by `find_vmt` is aligned.
    let mount_dlc_ebl = unsafe { ptr::read(&raw const vtable.as_ref().mount_dlc_ebl) };

    Ok(mount_dlc_ebl)
}
