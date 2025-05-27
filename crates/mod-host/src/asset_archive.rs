use std::{ffi::c_void, sync::Arc};

use me3_mod_host_assets::{
    mapping::ArchiveOverrideMapping,
    rva::{RVA_ASSET_LOOKUP, RVA_WWISE_ASSET_LOOKUP},
    string::DlUtf16String,
    wwise::{self, AkOpenMode},
};
use tracing::{debug, error};
use windows::{
    core::{PCSTR, PCWSTR},
    Win32::System::LibraryLoader::GetModuleHandleA,
};

use crate::host::ModHost;

/// This is a heavily obfuscated std::basic_string replace-esque. It's only
/// used when the game wants to expand a path as part of the archive lookup.
pub type ExpandArchivePathFn = extern "C" fn(*mut DlUtf16String, usize, usize, usize, usize, usize);

/// This function is part of FS's implementation of IAkLowLevelIOHook.
pub type WwisePathFn = extern "C" fn(usize, PCWSTR, u64, usize, usize, usize) -> usize;

pub fn attach(host: &mut ModHost, mapping: Arc<ArchiveOverrideMapping>) -> Result<(), eyre::Error> {
    let asset_override = {
        let mapping = mapping.clone();

        move |path: *mut DlUtf16String| {
            match unsafe { path.as_mut().map(DlUtf16String::get_mut) } {
                Some(Ok(path)) => {
                    let path_string = path.to_string();

                    debug!("Asset requested: {path_string}");

                    // Match the expanded path against the known overrides.
                    if let Some((mapped_path, mapped_override)) = mapping.get_override(&path_string)
                    {
                        // Replace the string with a canonical path to the asset if
                        // we did find an override. This will cause the game to
                        // pull the files bytes from the file system instead of the
                        // BDTs.
                        path.replace(mapped_override);

                        debug!(
                            "Supplied override: {path_string} -> {}",
                            mapped_path.display()
                        );
                    }
                }
                Some(Err(encoding)) => {
                    error!("Incorrect asset path encoding: {encoding:?}");
                }
                None => {
                    error!("Asset path was null!");
                }
            }
        }
    };

    host.hook(asset_hook_location())
        .with_closure(move |ctx, path, p2, p3, p4, p5, p6| {
            // Have the game expand the path for us.
            (ctx.trampoline)(path, p2, p3, p4, p5, p6);
            asset_override(path);
        })
        .install()?;

    host.hook(wwise_hook_location())
        .with_closure(move |ctx, p1, path, open_mode, p4, p5, p6| {
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
        .install()?;

    Ok(())
}

fn game_base() -> *const c_void {
    unsafe { GetModuleHandleA(PCSTR(std::ptr::null() as _)) }
        .expect("Could not retrieve game base for asset loader")
        .0
        .cast()
}

fn asset_hook_location() -> ExpandArchivePathFn {
    unsafe {
        std::mem::transmute::<*const c_void, ExpandArchivePathFn>(
            game_base().offset(RVA_ASSET_LOOKUP),
        )
    }
}

fn wwise_hook_location() -> WwisePathFn {
    unsafe {
        std::mem::transmute::<*const c_void, WwisePathFn>(
            game_base().offset(RVA_WWISE_ASSET_LOOKUP),
        )
    }
}

// static LOGFILE_HANDLE: LazyLock<Mutex<File>> = LazyLock::new(||
// Mutex::new(File::create("asset-hook.txt").unwrap()));
