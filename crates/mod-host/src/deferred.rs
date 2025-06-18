use std::{
    mem,
    sync::{LazyLock, Mutex},
};

use eyre::{eyre, OptionExt};
use tracing::{instrument, Level};
use windows::{
    core::{s, w},
    Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW},
};

use crate::host::ModHost;

type DeferredOnce = Option<Vec<Box<dyn FnOnce() + Send>>>;

static DEFERRED: Mutex<DeferredOnce> = Mutex::new(Some(Vec::new()));

/// Defers execution of a closure until after execution enters the game's WinMain.
///
/// Trying to defer a closure's execution after the point of initialization returns an error.
///
/// This implementation hooks `SteamAPI_SteamInit`.
#[instrument(skip_all, err)]
pub fn defer_until_init<F>(f: F) -> Result<(), eyre::Error>
where
    F: FnOnce() + Send + 'static,
{
    static HOOKED_STEAM_INIT: LazyLock<Result<(), eyre::Error>> = LazyLock::new(hook_steam_init);

    HOOKED_STEAM_INIT.as_ref().map_err(|e| eyre!(e))?;

    if let Some(deferred) = &mut *DEFERRED.lock().unwrap() {
        deferred.push(Box::new(f));

        Ok(())
    } else {
        Err(eyre!("tried to defer function after init"))
    }
}

#[instrument]
fn hook_steam_init() -> Result<(), eyre::Error> {
    ModHost::get_attached_mut()
        .hook(steam_init_fn()?)
        .with_closure(|trampoline| {
            let result = unsafe { trampoline() };

            if result {
                if let Some(deferred) = DEFERRED.lock().unwrap().take() {
                    deferred.into_iter().for_each(|f| f());
                }
            }

            result
        })
        .install()?;

    Ok(())
}

#[instrument(ret(level = Level::DEBUG))]
fn steam_init_fn() -> Result<unsafe extern "C" fn() -> bool, eyre::Error> {
    unsafe {
        let steam_dll = LoadLibraryW(w!("steam_api64.dll"))?;

        let steam_init =
            GetProcAddress(steam_dll, s!("SteamAPI_Init")).ok_or_eyre("SteamAPI_Init not found")?;

        Ok(mem::transmute(steam_init))
    }
}
