use std::{
    cell::OnceCell, fs::File, io::Write, sync::{Arc, LazyLock, Mutex}
};

use me3_mod_host_assets::{
    ffi::{get_dlwstring_contents, set_dlwstring_contents, DLWString},
    mapping::ArchiveOverrideMapping,
};
use tracing::info;

use crate::{
    detour::{Detour, DetourError},
    host::ModHost,
};

/// This is a heavily obfuscated std::basic_string replace-esque. It's only
/// used when the game wants to expand a path as part of the archive lookup.
pub type ExpandArchivePathFn = extern "C" fn(*mut DLWString, usize, usize, usize, usize, usize);

pub fn attach(host: &mut ModHost, mapping: ArchiveOverrideMapping) -> Result<(), DetourError> {
    let hook_instance: Arc<OnceCell<Arc<Detour<ExpandArchivePathFn>>>> = Default::default();

    let hook = {
        let hook_instance = hook_instance.clone();

        host.hook(get_hook_location())
            .with_closure(move |path, p2, p3, p4, p5, p6| {
                // Have the game expand the path for us.
                hook_instance.get().unwrap().trampoline()(path, p2, p3, p4, p5, p6);

                let resource_path_string =
                    get_dlwstring_contents(unsafe { path.as_mut().unwrap() });

                info!("Archive asset requested: {resource_path_string}");

                // Holy fuck this is cursed
                // LOGFILE_HANDLE.lock().unwrap()
                //     .write(format!("Archive asset requested: {resource_path_string}").as_bytes()).unwrap();

                // Match the expanded path against the known overrides.
                if let Some(mapped_override) = mapping.get_override(&resource_path_string) {
                    // Replace the string with a canonical path to the asset if
                    // we did find an override. This will cause the game to
                    // pull the files bytes from the file system instead of the
                    // BDTs.
                    set_dlwstring_contents(unsafe { path.as_ref().unwrap() }, mapped_override);

                    info!(
                        "Supplied override: {resource_path_string} -> {}",
                        unsafe { get_dlwstring_contents(path.as_ref().unwrap()) },
                    );
                }
            })
            .install()?
    };

    hook_instance.set(hook);

    Ok(())
}

fn get_hook_location() -> ExpandArchivePathFn {
    unsafe { std::mem::transmute::<usize, ExpandArchivePathFn>(0x14011e9c0) }
}

// static LOGFILE_HANDLE: LazyLock<Mutex<File>> = LazyLock::new(|| Mutex::new(File::open("lmao-log.txt").unwrap()));
