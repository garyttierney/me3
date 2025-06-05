use std::{
    collections::HashMap,
    ffi::OsString,
    mem,
    os::windows::ffi::OsStringExt,
    ptr::NonNull,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use eyre::OptionExt;
use me3_mod_host_assets::{
    dl_device::{self, DlDeviceManager, DlFileOperator, VfsMounts},
    ebl::EblFileManager,
    file_step,
    mapping::ArchiveOverrideMapping,
    singleton,
    string::DlUtf16String,
    wwise::{self, poll_wwise_open_file_fn, AkOpenMode},
};
use tracing::{debug, error, info};
use windows::{core::PCWSTR, Win32::System::LibraryLoader::GetModuleHandleW};

use crate::host::ModHost;

static VFS: Mutex<VfsMounts> = Mutex::new(VfsMounts::new());

pub fn attach_override(mapping: Arc<ArchiveOverrideMapping>) -> Result<(), eyre::Error> {
    let image_base = image_base();

    poll_singletons()?;
    debug!("Singleton initialization passed");

    // TODO: might want to freeze all threads here?

    hook_file_init(image_base, mapping.clone())?;

    hook_device_manager(image_base, mapping.clone())?;

    if let Err(e) = try_hook_wwise(mapping.clone()) {
        debug!("Skipping Wwise hook: {e}");
    }

    Ok(())
}

fn locate_device_manager(
    image_base: *const u8,
) -> Result<NonNull<DlDeviceManager>, dl_device::FindError> {
    struct DeviceManager(Result<NonNull<DlDeviceManager>, dl_device::FindError>);

    static DEVICE_MANAGER: OnceLock<DeviceManager> = OnceLock::new();

    unsafe impl Send for DeviceManager {}
    unsafe impl Sync for DeviceManager {}

    DEVICE_MANAGER
        .get_or_init(|| unsafe { DeviceManager(dl_device::find_device_manager(image_base)) })
        .0
        .clone()
}

fn hook_file_init(
    image_base: *const u8,
    mapping: Arc<ArchiveOverrideMapping>,
) -> Result<(), eyre::Error> {
    let device_manager = locate_device_manager(image_base)?;

    let init_fn = unsafe { file_step::find_init_fn(image_base)? };

    debug!("FileStep::STEP_Init found at {init_fn:?}");

    let hook_span = info_span!("hook");

    ModHost::get_attached_mut()
        .hook(init_fn)
        .with_closure(move |ctx, p1| {
            debug!("FileStep::STEP_Init called");

            let mut device_manager = DlDeviceManager::lock(device_manager);

            let snap = device_manager.snapshot();

            (ctx.trampoline)(p1);

            match snap {
                Ok(snap) => {
                    let new = device_manager.extract_new(snap);

                    debug!("Extracted BND4 mounts: {new:#?}");

                    let mut vfs = VFS.lock().unwrap();

                    *vfs = new;

                    if let Err(e) = hook_ebl_utility(image_base, mapping.clone()) {
                        error!("Failed to apply EBL hooks, restoring BND4 mounts; {e}");

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

fn hook_ebl_utility(
    image_base: *const u8,
    mapping: Arc<ArchiveOverrideMapping>,
) -> Result<(), eyre::Error> {
    let device_manager = locate_device_manager(image_base)?;

    let ebl_utility_vtable = unsafe { EblFileManager::ebl_utility_vtable(image_base)? };

    let mut mod_host = ModHost::get_attached_mut();

    mod_host
        .hook(ebl_utility_vtable.make_ebl_object)
        .with_closure(move |ctx, p1, path, p3| {
            let mut device_manager = DlDeviceManager::lock(device_manager);

            let path_cstr = PCWSTR::from_raw(path);
            let expanded = unsafe { device_manager.expand_path(path_cstr.as_wide()) };

            if let Some(expanded) = expanded.map(|ex| OsString::from_wide(&ex)) {
                let expanded = expanded.to_string_lossy();

                if mapping.get_override(&expanded).is_some() {
                    return None;
                }
            }

            let _guard = device_manager.push_vfs(&VFS.lock().unwrap());

            (ctx.trampoline)(p1, path, p3)
        })
        .install()?;

    mod_host
        .hook(ebl_utility_vtable.mount_ebl)
        .with_closure(move |ctx, p1, p2, p3, p4, p5, p6, p7| {
            let mut device_manager = DlDeviceManager::lock(device_manager);

            let snap = device_manager.snapshot();

            // Mount EBL but report failure, rationale below.
            // TODO: only "fail" EBLs from a list of wwise EBLs per game.
            let _result = (ctx.trampoline)(p1, p2, p3, p4, p5, p6, p7);

            match snap {
                Ok(snap) => {
                    let new = device_manager.extract_new(snap);

                    debug!("Extracted BND4 mounts: {new:#?}");

                    let mut vfs = VFS.lock().unwrap();

                    vfs.append(new);
                }
                Err(e) => error!("BND4 snapshot error: {e}"),
            }

            // There is a bug in ELDEN RING MagicOrchestra code where any absolute path not
            // starting with "sd:" causes an infinite loop trying to resolve the
            // root. Reporting the EBL as having failed to mount prevents it from
            // being added to the lookup list, leaving it empty so that branch not taken.
            false
        })
        .install()?;

    Ok(())
}

fn hook_device_manager(
    image_base: *const u8,
    mapping: Arc<ArchiveOverrideMapping>,
) -> Result<(), eyre::Error> {
    let device_manager = locate_device_manager(image_base)?;

    debug!("DLDeviceManager found at {device_manager:?}");

    let mut mod_host = ModHost::get_attached_mut();

    let open_disk_file = DlDeviceManager::lock(device_manager).open_disk_file_fn();

    let override_path = {
        let mapping = mapping.clone();

        move |path: &DlUtf16String| {
            let path = path.get().ok()?;

            let expanded = DlDeviceManager::lock(device_manager).expand_path(path.as_bytes());

            let expanded = expanded
                .map(|ex| OsString::from_wide(&ex))?
                .to_string_lossy()
                .to_string();

            debug!("[Asset] Requested \"{expanded}\"");

            let (mapped_path, mapped_override) = mapping.get_override(&expanded)?;

            info!("[Asset] Supplied override \"{mapped_path}\"");

            let mut path = path.clone();
            path.replace(mapped_override);

            Some(path)
        }
    };

    let hook_set_path = move |file_operator: NonNull<DlFileOperator>| {
        hook_set_path(image_base, file_operator, mapping.clone())
            .inspect_err(|e| error!("Failed to hook DLFileOperator::set_path: {e}"))
            .is_ok()
    };

    mod_host
        .hook(open_disk_file)
        .with_closure(move |ctx, p1, path, p3, p4, p5, p6| {
            let file_operator = if let Some(path) = override_path(unsafe { path.as_ref() }) {
                (ctx.trampoline)(
                    p1,
                    NonNull::from(&path).cast(),
                    path.as_ptr(),
                    p4,
                    p5.clone(),
                    p6,
                )
            } else {
                (ctx.trampoline)(p1, path, p3, p4, p5.clone(), p6)
            };

            if let Some(file_operator) = file_operator {
                static HOOK_RESULT: OnceLock<bool> = OnceLock::new();

                if *HOOK_RESULT.get_or_init(|| hook_set_path(file_operator)) {
                    return Some(file_operator);
                }
            }

            VFS.lock().unwrap().try_open_file(path, p3, p4, p5, p6)
        })
        .install()?;

    Ok(())
}

fn hook_set_path(
    image_base: *const u8,
    file_operator: NonNull<DlFileOperator>,
    mapping: Arc<ArchiveOverrideMapping>,
) -> Result<(), eyre::Error> {
    let set_path = unsafe { file_operator.as_ref().as_ref().set_path };

    let device_manager = locate_device_manager(image_base)?;

    let override_path = move |path: &DlUtf16String| {
        let path = path.get().ok()?;

        let expanded = DlDeviceManager::lock(device_manager).expand_path(path.as_bytes());

        let expanded = expanded
            .map(|ex| OsString::from_wide(&ex))?
            .to_string_lossy()
            .to_string();

        debug!("[Asset] Requested \"{expanded}\"");

        let (mapped_path, mapped_override) = mapping.get_override(&expanded)?;

        info!("[Asset] Supplied override \"{mapped_path}\"");

        let mut path = path.clone();
        path.replace(mapped_override);

        Some(path)
    };

    ModHost::get_attached_mut()
        .hook(set_path)
        .with_closure(move |ctx, p1, path, p3, p4| {
            if let Some(path) = override_path(unsafe { path.as_ref() }) {
                (ctx.trampoline)(p1, path.as_ref().into(), p3, p4)
            } else {
                (ctx.trampoline)(p1, path, p3, p4)
            }
        })
        .install()?;

    Ok(())
}

fn try_hook_wwise(mapping: Arc<ArchiveOverrideMapping>) -> Result<(), eyre::Error> {
    let wwise_open_file =
        poll_wwise_open_file_fn(Duration::from_millis(1), Duration::from_secs(5))?;

    ModHost::get_attached_mut()
        .hook(wwise_open_file)
        .with_closure(move |ctx, p1, path, open_mode, p4, p5, p6| {
            let path_string = unsafe { PCWSTR::from_raw(path).to_string().unwrap() };

            debug!("[Wwise] Requested \"{path_string}\"");

            if let Some((mapped_path, mapped_override)) =
                wwise::find_override(&mapping, &path_string)
            {
                info!("[Wwise] Supplied override \"{mapped_path}\"");

                // Force lookup to wwise's ordinary read (from disk) mode instead of the EBL read.
                (ctx.trampoline)(
                    p1,
                    mapped_override.as_ptr(),
                    AkOpenMode::Read as _,
                    p4,
                    p5,
                    p6,
                )
            } else {
                (ctx.trampoline)(p1, path, open_mode, p4, p5, p6)
            }
        })
        .install()?;

    Ok(())
}

fn image_base() -> *const u8 {
    unsafe { GetModuleHandleW(PCWSTR::null()) }
        .expect("GetModuleHandleW failed")
        .0 as *const u8
}

fn poll_singletons() -> Result<&'static HashMap<String, NonNull<*mut u8>>, eyre::Error> {
    singleton::poll_map(Duration::from_millis(1), Duration::from_secs(5))
        .ok_or_eyre("singleton mapping timed out; no singletons found")
}
