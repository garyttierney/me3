use std::{
    cell::OnceCell,
    fs::File,
    io::Write,
    sync::{Arc, LazyLock, Mutex},
};

use me3_mod_host_assets::{
    ffi::{get_dlwstring_contents, set_dlwstring_contents, DLWString},
    mapping::ArchiveOverrideMapping,
    rva::{RVA_ASSET_LOOKUP, RVA_WWISE_ASSET_LOOKUP},
    wwise::{self, AkOpenMode},
};
use tracing::debug;
use windows::{
    core::{PCSTR, PCWSTR},
    Win32::System::LibraryLoader::GetModuleHandleA,
};

use crate::{
    detour::{Detour, DetourError},
    host::ModHost,
};

/// This is a heavily obfuscated std::basic_string replace-esque. It's only
/// used when the game wants to expand a path as part of the archive lookup.
pub type ExpandArchivePathFn = extern "C" fn(*mut DLWString, usize, usize, usize, usize, usize);

/// This function is part of FS's implementation of IAkLowLevelIOHook.
pub type WwisePathFn = extern "C" fn(usize, PCWSTR, u64, usize, usize, usize) -> usize;

pub fn attach(host: &mut ModHost, mapping: Arc<ArchiveOverrideMapping>) -> Result<(), DetourError> {
    let asset_hook_instance: Arc<OnceCell<Arc<Detour<ExpandArchivePathFn>>>> = Default::default();
    let asset_hook = {
        let mapping = mapping.clone();

        host.hook(asset_hook_location())
            .with_closure(move |ctx, path, p2, p3, p4, p5, p6| {
                // Have the game expand the path for us.
                (ctx.trampoline)(path, p2, p3, p4, p5, p6);

                let resource_path_string =
                    get_dlwstring_contents(unsafe { path.as_mut().unwrap() });

                debug!("Asset requested: {resource_path_string}");

                // Match the expanded path against the known overrides.
                if let Some(mapped_override) = mapping.get_override(&resource_path_string) {
                    // Replace the string with a canonical path to the asset if
                    // we did find an override. This will cause the game to
                    // pull the files bytes from the file system instead of the
                    // BDTs.
                    set_dlwstring_contents(unsafe { path.as_ref().unwrap() }, mapped_override);

                    debug!("Supplied override: {resource_path_string} -> {}", unsafe {
                        get_dlwstring_contents(path.as_ref().unwrap())
                    });
                }
            })
            .install()?
    };

    let wwise_hook = {
        let mapping = mapping.clone();

        host.hook(wwise_hook_location())
            .with_closure(move |ctx, p1, path, mut open_mode, p4, p5, p6| {
                let path_string = unsafe { path.to_string().unwrap() };

                debug!("Wwise asset requested: {path_string}");

                if let Some(mapped_override) = wwise::find_override(&mapping, &path_string) {
                    debug!("Supplied override for {path_string}");
                    // Force lookup to wwise'ordinary read (from disk) mode instead of the EBL read.
                    (ctx.trampoline)(
                        p1,
                        PCWSTR(mapped_override.as_ptr()),
                        AkOpenMode::Read as _,
                        p4,
                        p5,
                        p6,
                    )
                } else {
                    (ctx.trampoline)(p1, path, open_mode, p4, p5, p6)
                }
            })
            .install()?
    };

    Ok(())
}

fn game_base() -> isize {
    unsafe { GetModuleHandleA(PCSTR(std::ptr::null() as _)) }
        .expect("Could not retrieve game base for asset loader")
        .0
}

fn asset_hook_location() -> ExpandArchivePathFn {
    unsafe {
        std::mem::transmute::<isize, ExpandArchivePathFn>(game_base() + RVA_ASSET_LOOKUP as isize)
    }
}

fn wwise_hook_location() -> WwisePathFn {
    unsafe {
        std::mem::transmute::<isize, WwisePathFn>(game_base() + RVA_WWISE_ASSET_LOOKUP as isize)
    }
}

// static LOGFILE_HANDLE: LazyLock<Mutex<File>> = LazyLock::new(||
// Mutex::new(File::create("asset-hook.txt").unwrap()));
