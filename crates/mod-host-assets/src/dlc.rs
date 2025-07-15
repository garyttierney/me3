use me3_binary_analysis::rtti::ClassMap;

type MountDlcEbl = unsafe extern "C" fn(usize, bool, usize, usize);

pub fn mount_dlc_ebl(class_map: &ClassMap) -> Option<MountDlcEbl> {
    #[repr(C)]
    struct Vtable {
        other: [usize; 8],
        mount_dlc_ebl: MountDlcEbl,
    }

    let vtable = class_map
        .get("CS::CSDlcPlatformImp_forSteam")?
        .first()?
        .as_ptr::<Vtable>();

    // SAFETY: vtable pointer in `class_map` is aligned.
    let mount_dlc_ebl = unsafe { std::ptr::read(&raw const vtable.as_ref()?.mount_dlc_ebl) };

    Some(mount_dlc_ebl)
}
