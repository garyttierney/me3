use std::{
    mem,
    sync::{LazyLock, Mutex, Once},
};

use eyre::{eyre, OptionExt};
use tracing::{info, instrument, Level, Span};
use windows::{
    core::{s, w},
    Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW},
};

use crate::host::ModHost;

pub enum Deferred {
    BeforeMain,
    AfterMain,
}

type DeferredOnce = Option<Vec<Box<dyn FnOnce() + Send>>>;

static DEFERRED_BEFORE_MAIN: Mutex<DeferredOnce> = Mutex::new(Some(Vec::new()));
static DEFERRED_AFTER_MAIN: Mutex<DeferredOnce> = Mutex::new(Some(Vec::new()));

/// Defers execution of a closure.
///
/// Trying to defer a closure's execution after the point of initialization returns an error.
#[instrument(skip_all, err)]
pub fn defer_init<F>(span: Span, until: Deferred, f: F) -> Result<(), eyre::Error>
where
    F: FnOnce() + Send + 'static,
{
    let deferred = match until {
        Deferred::BeforeMain => {
            static SCHEDULED_AFTER_ARXAN: Once = Once::new();
            SCHEDULED_AFTER_ARXAN.call_once(schedule_after_arxan);

            &DEFERRED_BEFORE_MAIN
        }
        Deferred::AfterMain => {
            static HOOKED_STEAM_INIT: LazyLock<Result<(), eyre::Error>> =
                LazyLock::new(hook_steam_init);

            HOOKED_STEAM_INIT.as_ref().map_err(|e| eyre!(e))?;

            &DEFERRED_AFTER_MAIN
        }
    };

    deferred
        .lock()
        .unwrap()
        .as_mut()
        .map(|deferred| deferred.push(Box::new(move || span.in_scope(f))))
        .ok_or_eyre("tried to defer function after init")
}

#[instrument]
fn hook_steam_init() -> Result<(), eyre::Error> {
    ModHost::get_attached()
        .hook(steam_init_fn()?)
        .with_closure(|trampoline| {
            let result = unsafe { trampoline() };

            if result && let Some(deferred) = DEFERRED_AFTER_MAIN.lock().unwrap().take() {
                deferred.into_iter().for_each(|f| f());
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

#[instrument]
fn schedule_after_arxan() {
    let deferred = || {
        if let Some(deferred) = DEFERRED_BEFORE_MAIN.lock().unwrap().take() {
            deferred.into_iter().for_each(|f| f());
        }
    };

    if ModHost::get_attached().disable_arxan {
        let span = Span::current();
        unsafe {
            dearxan::disabler::neuter_arxan(move |result| {
                span.in_scope(|| info!(?result));
                deferred();
            });
        }
    } else {
        unsafe {
            dearxan::disabler::schedule_after_arxan(move |_, _| deferred());
        }
    }
}
