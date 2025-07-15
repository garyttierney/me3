use std::{
    ffi::OsString,
    mem,
    os::windows::ffi::OsStringExt,
    ptr::NonNull,
    sync::{Arc, Mutex, OnceLock},
};

use eyre::{eyre, OptionExt};
use me3_binary_analysis::rtti::ClassMap;
use me3_mod_host_assets::{
    dl_device::{self, DlDeviceManager, DlFileOperator, VfsMounts},
    dlc::mount_dlc_ebl,
    ebl::EblFileManager,
    file_step,
    mapping::ArchiveOverrideMapping,
    string::DlUtf16String,
    wwise::{self, find_wwise_open_file, AkOpenMode},
};
use me3_mod_protocol::Game;
use tracing::{debug, error, info, info_span, instrument, warn};
use windows::core::PCWSTR;

use crate::{executable::Executable, host::ModHost};

static VFS: Mutex<VfsMounts> = Mutex::new(VfsMounts::new());

#[instrument(name = "assets", skip_all)]
pub fn attach_override(
    _game: Game,
    exe: Executable,
    class_map: Arc<ClassMap>,
    mapping: Arc<ArchiveOverrideMapping>,
) -> Result<(), eyre::Error> {
    hook_file_init(exe, class_map.clone(), mapping.clone())?;

    if let Err(e) = try_hook_wwise(exe, &class_map, mapping.clone()) {
        debug!("error" = &*e, "skipping Wwise hook");
    }

    if let Err(e) = try_hook_dlc(exe, &class_map) {
        debug!("error" = &*e, "skipping DLC hook");
    }

    Ok(())
}

#[instrument(name = "file_step", skip_all)]
fn hook_file_init(
    exe: Executable,
    class_map: Arc<ClassMap>,
    mapping: Arc<ArchiveOverrideMapping>,
) -> Result<(), eyre::Error> {
    let init_fn = file_step::find_init_fn(exe)?;

    debug!("FileStep::STEP_Init" = ?init_fn);

    ModHost::get_attached_mut()
        .hook(init_fn)
        .with_span(info_span!("hook"))
        .with_closure(move |p1, trampoline| {
            let mut device_manager = match locate_device_manager(exe) {
                Ok(device_manager) => DlDeviceManager::lock(device_manager),
                Err(e) => {
                    error!("error" = &*eyre!(e), "failed to locate device manager");

                    unsafe {
                        trampoline(p1);
                    }

                    return;
                }
            };

            if let Err(e) = hook_device_manager(exe, mapping.clone()) {
                error!("error" = &*eyre!(e), "failed to hook device manager");

                unsafe {
                    trampoline(p1);
                }

                return;
            }

            let snap = device_manager.snapshot();

            unsafe {
                trampoline(p1);
            }

            match snap {
                Ok(snap) => {
                    let new = device_manager.extract_new(snap);

                    debug!("extracted_mounts" = ?new);

                    let mut vfs = VFS.lock().unwrap();

                    *vfs = new;

                    if let Err(e) = hook_ebl_utility(exe, &class_map, mapping.clone()) {
                        error!("error" = &*e, "failed to apply EBL hooks");

                        let vfs = mem::take(&mut *vfs);

                        let guard = device_manager.push_vfs(&vfs);

                        mem::forget(guard);
                    }
                }
                Err(e) => error!("BND4 snapshot error: {e}"),
            }
        })
        .install()?;

    Ok(())
}

#[instrument(name = "ebl", skip_all)]
fn hook_ebl_utility(
    exe: Executable,
    class_map: &ClassMap,
    mapping: Arc<ArchiveOverrideMapping>,
) -> Result<(), eyre::Error> {
    let device_manager = locate_device_manager(exe)?;

    let make_ebl_object =
        EblFileManager::make_ebl_object(exe, class_map).ok_or_eyre("MakeEblObject not found")?;

    debug!(?make_ebl_object);

    let mut mod_host = ModHost::get_attached_mut();

    mod_host
        .hook(make_ebl_object)
        .with_closure(move |p1, path, p3, trampoline| {
            let mut device_manager = DlDeviceManager::lock(device_manager);

            let path_cstr = PCWSTR::from_raw(path);
            let expanded = unsafe { device_manager.expand_path(path_cstr.as_wide()) };

            if mapping
                .vfs_override(OsString::from_wide(&expanded))
                .is_some()
            {
                return None;
            }

            let _guard = device_manager.push_vfs(&VFS.lock().unwrap());

            unsafe { (trampoline)(p1, path, p3) }
        })
        .install()?;

    info!("applied asset override hook");

    Ok(())
}

#[instrument(name = "device_manager", skip_all)]
fn hook_device_manager(
    exe: Executable,
    mapping: Arc<ArchiveOverrideMapping>,
) -> Result<(), eyre::Error> {
    let device_manager = locate_device_manager(exe)?;

    let open_disk_file = DlDeviceManager::lock(device_manager).open_disk_file();

    let override_path = {
        let mapping = mapping.clone();

        move |path: &DlUtf16String| {
            let path = path.get().ok()?;
            let expanded = DlDeviceManager::lock(device_manager).expand_path(path.as_bytes());

            let (mapped_path, mapped_override) =
                mapping.vfs_override(OsString::from_wide(&expanded))?;

            info!("override" = mapped_path);

            let mut path = path.clone();
            path.replace(mapped_override);

            Some(path)
        }
    };

    let hook_set_path = move |file_operator: NonNull<DlFileOperator>| {
        hook_set_path(exe, file_operator, mapping.clone())
            .inspect_err(|e| error!("Failed to hook DLFileOperator::SetPath: {e}"))
            .is_ok()
    };

    ModHost::get_attached_mut()
        .hook(open_disk_file)
        .with_span(info_span!("hook"))
        .with_closure(move |p1, path, p3, p4, p5, p6, trampoline| {
            let file_operator = if let Some(path) = override_path(unsafe { path.as_ref() }) {
                unsafe {
                    trampoline(
                        p1,
                        NonNull::from(&path).cast(),
                        path.as_ptr(),
                        p4,
                        p5.clone(),
                        p6,
                    )
                }
            } else {
                unsafe { trampoline(p1, path, p3, p4, p5.clone(), p6) }
            };

            if let Some(file_operator) = file_operator {
                static HOOK_RESULT: OnceLock<bool> = OnceLock::new();

                if *HOOK_RESULT.get_or_init(|| hook_set_path(file_operator)) {
                    return Some(file_operator);
                }
            }

            unsafe { VFS.lock().unwrap().try_open_file(path, p3, p4, p5, p6) }
        })
        .install()?;

    info!("applied asset override hook");

    Ok(())
}

fn hook_set_path(
    exe: Executable,
    file_operator: NonNull<DlFileOperator>,
    mapping: Arc<ArchiveOverrideMapping>,
) -> Result<(), eyre::Error> {
    let vtable = unsafe { file_operator.as_ref().as_ref() };

    let device_manager = locate_device_manager(exe)?;

    let override_path = move |path: &DlUtf16String| {
        let path = path.get().ok()?;

        let expanded = DlDeviceManager::lock(device_manager).expand_path(path.as_bytes());

        let (_, mapped_override) = mapping.vfs_override(OsString::from_wide(&expanded))?;

        let mut path = path.clone();
        path.replace(mapped_override);

        Some(path)
    };

    for set_path in [vtable.set_path, vtable.set_path2, vtable.set_path3] {
        let override_path = override_path.clone();

        ModHost::get_attached_mut()
            .hook(set_path)
            .with_closure(move |p1, path, p3, p4, trampoline| {
                if let Some(path) = override_path(unsafe { path.as_ref() }) {
                    unsafe { trampoline(p1, path.as_ref().into(), p3, p4) }
                } else {
                    unsafe { trampoline(p1, path, p3, p4) }
                }
            })
            .install()?;
    }

    Ok(())
}

#[instrument(name = "wwise", skip_all)]
fn try_hook_wwise(
    exe: Executable,
    class_map: &ClassMap,
    mapping: Arc<ArchiveOverrideMapping>,
) -> Result<(), eyre::Error> {
    let wwise_open_file =
        find_wwise_open_file(exe, class_map).ok_or_eyre("WwiseOpenFileByName not found")?;

    ModHost::get_attached_mut()
        .hook(wwise_open_file)
        .with_span(info_span!("hook"))
        .with_closure(move |p1, path, open_mode, p4, p5, p6, trampoline| {
            let path_string = unsafe { PCWSTR::from_raw(path).to_string().unwrap() };
            debug!("asset" = path_string);

            if let Some((mapped_path, mapped_override)) =
                wwise::find_override(&mapping, &path_string)
            {
                info!("override" = mapped_path);

                // Force lookup to wwise's ordinary read (from disk) mode instead of the EBL read.
                unsafe {
                    trampoline(
                        p1,
                        mapped_override.as_ptr(),
                        AkOpenMode::Read as _,
                        p4,
                        p5,
                        p6,
                    )
                }
            } else {
                unsafe { trampoline(p1, path, open_mode, p4, p5, p6) }
            }
        })
        .install()?;

    info!("applied asset override hook");

    Ok(())
}

#[instrument(name = "dlc", skip_all)]
fn try_hook_dlc(exe: Executable, class_map: &ClassMap) -> Result<(), eyre::Error> {
    let mount_dlc_ebl = mount_dlc_ebl(class_map).ok_or_eyre("MountDlcEbl not found")?;

    ModHost::get_attached_mut()
        .hook(mount_dlc_ebl)
        .with_closure(move |p1, p2, p3, p4, trampoline| {
            if let Ok(device_manager) = locate_device_manager(exe) {
                let mut device_manager = DlDeviceManager::lock(device_manager);

                let snap = device_manager.snapshot();

                unsafe {
                    trampoline(p1, p2, p3, p4);
                }

                match snap {
                    Ok(snap) => {
                        let new = device_manager.extract_new(snap);

                        if !new.is_empty() {
                            debug!("extracted_mounts" = ?new);

                            let mut vfs = VFS.lock().unwrap();

                            vfs.append(new);
                        }
                    }
                    Err(e) => error!("BND4 snapshot error: {e}"),
                }

                return;
            }

            unsafe {
                trampoline(p1, p2, p3, p4);
            }
        })
        .install()?;

    info!("applied asset override hook");

    Ok(())
}

fn locate_device_manager(
    exe: Executable,
) -> Result<NonNull<DlDeviceManager>, dl_device::FindError> {
    struct DeviceManager(Result<NonNull<DlDeviceManager>, dl_device::FindError>);

    static DEVICE_MANAGER: OnceLock<DeviceManager> = OnceLock::new();

    unsafe impl Send for DeviceManager {}
    unsafe impl Sync for DeviceManager {}

    DEVICE_MANAGER
        .get_or_init(|| DeviceManager(dl_device::find_device_manager(exe)))
        .0
        .clone()
}
