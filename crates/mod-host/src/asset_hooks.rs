use std::{
    alloc::{GlobalAlloc, Layout},
    ffi::OsString,
    fs::OpenOptions,
    io::{self, Read, Seek, Write},
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::Path,
    ptr::NonNull,
    slice,
    sync::{Arc, Mutex, OnceLock},
};

use eyre::{eyre, OptionExt};
use me3_binary_analysis::{fd4_step::Fd4StepTables, rtti::ClassMap};
use me3_launcher_attach_protocol::AttachConfig;
use me3_mod_host_assets::{
    bhd5::Bhd5Header,
    dl_device::{self, DlDeviceManager, DlFileOperator, VfsMounts},
    ebl::{mount_ebl, DlDeviceEblExt, EblFileManager},
    mapping::ArchiveOverrideMapping,
    wwise::{self, find_wwise_open_file, AkOpenMode},
};
use me3_mod_host_types::{alloc::DlStdAllocator, string::DlUtf16String};
use me3_mod_protocol::Game;
use rdvec::{RawVec, Vec as DynVec};
use rsa::{pkcs1::DecodeRsaPublicKey, traits::PublicKeyParts, RsaPublicKey};
use tempfile::NamedTempFile;
use tracing::{debug, error, info, info_span, instrument, warn};
use windows::core::{PCSTR, PCWSTR};
use xxhash_rust::xxh3;

use crate::{executable::Executable, host::ModHost};

static VFS_MOUNTS: Mutex<VfsMounts> = Mutex::new(VfsMounts::new());

#[instrument(name = "assets", skip_all)]
pub fn attach_override(
    attach_config: Arc<AttachConfig>,
    exe: Executable,
    class_map: Arc<ClassMap<'static>>,
    step_tables: &Fd4StepTables,
    mapping: Arc<ArchiveOverrideMapping>,
) -> Result<(), eyre::Error> {
    enable_loose_params(&attach_config, &mapping);

    hook_file_init(
        attach_config,
        exe,
        class_map.clone(),
        step_tables,
        mapping.clone(),
    )?;

    if let Err(e) = try_hook_wwise(exe, &class_map, mapping.clone()) {
        debug!("error" = &*e, "skipping Wwise hook");
    }

    Ok(())
}

fn enable_loose_params(attach_config: &AttachConfig, mapping: &ArchiveOverrideMapping) {
    // Some Dark Souls 3 mods use a legacy Mod Engine 2 option of loading "loose" param files
    // instead of Data0. For backwards compatibility me3 enables it below.
    if attach_config.game != Game::DarkSouls3 {
        return;
    }

    static LOOSE_PARAM_FILES: [&str; 3] = [
        "data1:/param/gameparam/gameparam.parambnd.dcx",
        "data1:/param/gameparam/gameparam_dlc1.parambnd.dcx",
        "data1:/param/gameparam/gameparam_dlc2.parambnd.dcx",
    ];

    if LOOSE_PARAM_FILES
        .iter()
        .any(|file| mapping.vfs_override(file).is_some())
    {
        ModHost::get_attached().override_game_property("Game.Debug.EnableRegulationFile", false);
    }
}

#[instrument(name = "file_step", skip_all)]
fn hook_file_init(
    attach_config: Arc<AttachConfig>,
    exe: Executable,
    class_map: Arc<ClassMap<'static>>,
    step_tables: &Fd4StepTables,
    mapping: Arc<ArchiveOverrideMapping>,
) -> Result<(), eyre::Error> {
    let init_fn = step_tables
        .by_name("CSFileStep::STEP_Init")
        .or_else(|| step_tables.by_name("SprjFileStep::STEP_Init"))
        .ok_or_eyre("FileStep::STEP_Init not found")?;

    debug!("FileStep::STEP_Init" = ?init_fn);

    ModHost::get_attached()
        .hook(init_fn)
        .with_span(info_span!("hook"))
        .with_closure(move |p1, trampoline| {
            let result = hook_device_manager(exe, mapping.clone())
                .and_then(|_| hook_mount_ebl(attach_config.clone(), exe))
                .inspect_err(|e| error!("error" = &**e, "failed apply pre-hooks"));

            unsafe {
                trampoline(p1);
            }

            if result.is_ok()
                && let Err(e) = hook_ebl_utility(exe, &class_map, mapping.clone())
            {
                error!("error" = &*e, "failed to apply post-hooks");
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

    ModHost::get_attached()
        .hook(make_ebl_object)
        .with_closure(move |p1, path, p3, trampoline| {
            let mut device_manager = DlDeviceManager::lock(device_manager);

            let expanded = unsafe { device_manager.expand_path(path.as_wide()) };

            if mapping
                .vfs_override(OsString::from_wide(&expanded))
                .is_some()
            {
                return None;
            }

            let _guard = device_manager.push_vfs_mounts(&VFS_MOUNTS.lock().unwrap());

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
            let expanded = DlDeviceManager::lock(device_manager).expand_path(path.as_slice());

            let (mapped_path, mapped_override) =
                mapping.vfs_override(OsString::from_wide(&expanded))?;

            info!("override" = mapped_path);

            let mut path = path.clone();

            path.replace_from_slice(mapped_override);

            Some(path)
        }
    };

    let hook_set_path = move |file_operator: NonNull<DlFileOperator>| {
        hook_set_path(exe, file_operator, mapping.clone())
            .inspect_err(|e| error!("Failed to hook DLFileOperator::SetPath: {e}"))
            .is_ok()
    };

    ModHost::get_attached()
        .hook(open_disk_file)
        .with_span(info_span!("hook"))
        .with_closure(move |p1, path, p3, p4, p5, p6, trampoline| {
            let file_operator = if let Some(path) = override_path(unsafe { path.as_ref() }) {
                unsafe {
                    trampoline(
                        p1,
                        NonNull::from(&path).cast(),
                        PCWSTR::from_raw(path.as_ptr()),
                        p4,
                        p5,
                        p6,
                    )
                }
            } else {
                unsafe { trampoline(p1, path, p3, p4, p5, p6) }
            };

            if let Some(file_operator) = file_operator {
                static HOOK_RESULT: OnceLock<bool> = OnceLock::new();

                if *HOOK_RESULT.get_or_init(|| hook_set_path(file_operator)) {
                    return Some(file_operator);
                }
            }

            unsafe {
                VFS_MOUNTS
                    .lock()
                    .unwrap()
                    .try_open_file(path, p3, p4, p5, p6)
            }
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

        let expanded = DlDeviceManager::lock(device_manager).expand_path(path.as_slice());

        let (_, mapped_override) = mapping.vfs_override(OsString::from_wide(&expanded))?;

        let mut path = path.clone();

        path.replace_from_slice(mapped_override);

        Some(path)
    };

    for set_path in [vtable.set_path, vtable.set_path2, vtable.set_path3] {
        let override_path = override_path.clone();

        ModHost::get_attached()
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

#[instrument(name = "mount_ebl", skip_all)]
fn hook_mount_ebl(attach_config: Arc<AttachConfig>, exe: Executable) -> Result<(), eyre::Error> {
    fn load_cached_ebl<P, F>(
        exe: Executable,
        cache_path: P,
        bhd_path: PCWSTR,
        key_c_str: PCSTR,
        allocator: DlStdAllocator,
        trampoline: F,
    ) -> Result<(), eyre::Error>
    where
        P: AsRef<Path>,
        F: Fn(PCWSTR) -> bool,
    {
        let mut device_manager = DlDeviceManager::lock(locate_device_manager(exe)?);

        let expanded = unsafe { device_manager.expand_path(bhd_path.as_wide()) };
        let bhd_path = OsString::from_wide(&expanded);

        // Parse the public RSA key to know the block size for decryption.
        let pub_key = key_from_pem_c_str(key_c_str)?;

        // Read the original file for hashing to use as the cached file name.
        let original = Arc::new(std::fs::read(&bhd_path)?);

        let hash = std::thread::spawn({
            let original = original.clone();
            move || xxh3::xxh3_128(&original)
        });

        // Write a temporary file with the size of a single block and have the
        // game decrypt it, which creates an EblFileDevice and lets us read
        // the original file size.
        let mut stub_file = NamedTempFile::new_in(cache_path.as_ref())?;

        stub_file.write_all(&original[..Ord::min(pub_key.size(), original.len())])?;

        let snap = device_manager.snapshot()?;

        invoke_trampoline(&trampoline, &stub_file)?;

        let new_mounts = device_manager.extract_new(snap);

        let mut device = new_mounts
            .devices()
            .next()
            .ok_or_eyre("no devices were added")?;

        let original_len = unsafe {
            device
                .as_ref()
                .as_bhd_holder_unchecked()
                .bhd_header()
                .map(Bhd5Header::file_size)
        };

        let hash = hash.join().expect("thread panicked");

        let cached_bhd_path = cache_path.as_ref().join(format!("{hash:032x?}.bhd"));

        // Create or open the cache file.
        let mut cached = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(cached_bhd_path)?;

        let cached_len = cached.seek(io::SeekFrom::End(0))? as usize;
        cached.seek(io::SeekFrom::Start(0))?;

        // If the length of the cached file is zero the file has just been created.
        // If the length does not match there was an error writing the cache or a hash collision.
        if cached_len == 0 || original_len.is_some_and(|len| cached_len != len as usize) {
            // Clear the file and let the game decrypt the original, before caching it.
            cached.set_len(0)?;

            let snap = device_manager.snapshot()?;

            invoke_trampoline(&trampoline, &bhd_path)?;

            let new_mounts = device_manager.extract_new(snap);

            let device = new_mounts
                .devices()
                .next()
                .ok_or_eyre("no devices were added")?;

            let header = unsafe {
                device
                    .as_ref()
                    .as_bhd_holder_unchecked()
                    .bhd_header()
                    .ok_or_eyre("BHD header is null")?
            };

            VFS_MOUNTS.lock().unwrap().append(new_mounts);

            // Successfully mounted the ebl, do not report subsequent caching errors.
            let _ = cached
                .write_all(header.as_slice())
                .and_then(|_| cached.flush());
        } else {
            // Opened a cached decrypted file, read and assign its contents.
            // Use the game's own allocator as it will be freed with it later.
            let buf = unsafe {
                let ptr = NonNull::new(
                    allocator.alloc(Layout::from_size_align_unchecked(cached_len, 4096)),
                )
                .ok_or_eyre("failed to allocate buffer for cached file")?;

                slice::from_raw_parts_mut(ptr.as_ptr(), cached_len)
            };

            cached.read_exact(buf).inspect_err(|_| unsafe {
                allocator.dealloc(
                    buf.as_mut_ptr(),
                    Layout::from_size_align_unchecked(cached_len, 4096),
                );
            })?;

            unsafe {
                device
                    .as_mut()
                    .as_mut_bhd_holder_unchecked()
                    .assign_bhd_contents(buf.as_mut_ptr().cast());
            }

            VFS_MOUNTS.lock().unwrap().append(new_mounts);
        }

        Ok(())
    }

    fn key_from_pem_c_str(key_c_str: PCSTR) -> Result<RsaPublicKey, eyre::Error> {
        let key_str = unsafe { str::from_utf8(key_c_str.as_bytes())? };

        let mut lines = key_str.lines();

        let mut normalized = String::new();

        let pre = lines.next().ok_or_eyre("malformed PEM")?;

        normalized.push_str(pre);
        normalized.push('\n');

        let post = lines.next_back().ok_or_eyre("malformed PEM")?;

        let is_base64char = |c: &char| c.is_ascii_alphanumeric() | ['+', '/', '='].contains(c);

        for line in lines {
            normalized.extend(line.chars().filter(is_base64char));
            normalized.push('\n');
        }

        normalized.push_str(post);

        let pub_key = RsaPublicKey::from_pkcs1_pem(&normalized)?;

        Ok(pub_key)
    }

    fn invoke_trampoline<S, F>(trampoline: &F, bhd_path: S) -> Result<(), eyre::Error>
    where
        S: AsRef<Path>,
        F: Fn(PCWSTR) -> bool,
    {
        let mut bhd_path = bhd_path.as_ref().to_owned().into_os_string();

        bhd_path.push("\0");
        let bhd_path = bhd_path.encode_wide().collect::<Vec<_>>();

        match trampoline(PCWSTR::from_raw(bhd_path.as_ptr())) {
            true => Ok(()),
            false => Err(eyre!("trampoline returned null")),
        }
    }

    let mount_ebl = mount_ebl(exe).ok_or_eyre("MountEbl not found")?;

    debug!(?mount_ebl);

    ModHost::get_attached()
        .hook(mount_ebl)
        .with_span(info_span!("hook"))
        .with_closure(move |p1, p2, p3, p4, p5, p6, trampoline| {
            if attach_config.boot_boost && let Some(cache_path) = &attach_config.cache_path {
                match load_cached_ebl(exe, cache_path, p2, p5, p4, |p2| unsafe {
                    trampoline(p1, p2, p3, p4, p5, p6)
                }) {
                    Ok(()) => {
                        return true;
                    }
                    Err(e) => {
                        error!("error" = &*e, key = %unsafe { str::from_utf8(p5.as_bytes()).unwrap() });
                    }
                }
            }

            if let Ok(device_manager) = locate_device_manager(exe) {
                let mut device_manager = DlDeviceManager::lock(device_manager);

                let snap = device_manager.snapshot();

                let result = unsafe { trampoline(p1, p2, p3, p4, p5, p6) };

                match snap {
                    Ok(snap) => {
                        let new = device_manager.extract_new(snap);
                        VFS_MOUNTS.lock().unwrap().append(new);
                    }
                    Err(e) => error!("error" = &*eyre!(e), "snapshot error"),
                }

                result
            } else {
                unsafe { trampoline(p1, p2, p3, p4, p5, p6) }
            }
        })
        .install()?;

    info!("applied asset override hook");

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

    ModHost::get_attached()
        .hook(wwise_open_file)
        .with_span(info_span!("hook"))
        .with_closure(move |p1, path, open_mode, p4, p5, p6, trampoline| {
            let path_string = unsafe { path.to_string().unwrap() };
            debug!("asset" = path_string);

            if let Some((mapped_path, mapped_override)) =
                wwise::find_override(&mapping, &path_string)
            {
                info!("override" = mapped_path);

                // Force lookup to wwise's ordinary read (from disk) mode instead of the EBL read.
                unsafe {
                    trampoline(
                        p1,
                        PCWSTR::from_raw(mapped_override.as_ptr()),
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
