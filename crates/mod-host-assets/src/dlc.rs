use me3_binary_analysis::rtti::ClassMap;

type MountDlcEbl = unsafe extern "C" fn(usize, bool, usize, usize);

pub fn mount_dlc_ebl(class_map: &ClassMap) -> Option<MountDlcEbl> {
    #[repr(C)]
    struct Vtable {
        other: [usize; 8],
        mount_dlc_ebl: MountDlcEbl,
    }

    let mount_dlc_ebl = unsafe {
        class_map
            .get("CS::CSDlcPlatformImp_forSteam")?
            .first()?
            .as_ref::<Vtable>()
            .mount_dlc_ebl
    };

    Some(mount_dlc_ebl)
}
