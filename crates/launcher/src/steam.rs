use std::{mem, path::Path};

use eyre::OptionExt;
use tracing::instrument;
use windows::{
    core::{s, HSTRING},
    Win32::{
        Foundation::FreeLibrary,
        System::LibraryLoader::{GetProcAddress, LoadLibraryW},
    },
};

use crate::LauncherResult;

#[instrument(skip_all, err)]
pub fn require_steam<P: AsRef<Path>>(game_binary: P) -> LauncherResult<()> {
    let steam_dll = game_binary.as_ref().with_file_name("steam_api64.dll");

    let handle = unsafe { LoadLibraryW(&HSTRING::from(&*steam_dll))? };

    let steam_api_init = unsafe {
        mem::transmute::<_, extern "system" fn() -> bool>(
            GetProcAddress(handle, s!("SteamAPI_Init"))
                .ok_or_eyre("SteamAPI_Init export not found")?,
        )
    };

    let result = steam_api_init()
        .then_some(())
        .ok_or_eyre("Steam is required to run this game");

    unsafe { FreeLibrary(handle)? };

    result
}
