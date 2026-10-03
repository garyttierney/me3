use std::sync::{LazyLock, Mutex, Once};

use diversion::hook::{custom::install_custom, leak::StaticHook};
use eyre::{eyre, Context, ContextCompat, OptionExt};
use pelite::pe::{Pe, Rva, Va};
use tracing::{info, instrument, Level, Span};

use crate::{executable::Executable, hook::install_iat, host::ModHost};

pub enum Deferred {
    BeforeMain,
    AfterMain,
    AfterSysPropsInit,
    AfterDbgPropsInit,
}

type DeferredOnce = Option<Vec<Box<dyn FnOnce() + Send>>>;

static BEFORE_MAIN: Mutex<DeferredOnce> = Mutex::new(Some(Vec::new()));
static AFTER_MAIN: Mutex<DeferredOnce> = Mutex::new(Some(Vec::new()));
static AFTER_SYS_PROPS_INIT: Mutex<DeferredOnce> = Mutex::new(Some(Vec::new()));
static AFTER_DBG_PROPS_INIT: Mutex<DeferredOnce> = Mutex::new(Some(Vec::new()));

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

            &BEFORE_MAIN
        }
        Deferred::AfterMain => {
            static HOOKED_STEAM_INIT: LazyLock<Result<(), eyre::Error>> =
                LazyLock::new(hook_steam_init);

            HOOKED_STEAM_INIT.as_ref().map_err(|e| eyre!(e))?;

            &AFTER_MAIN
        }
        Deferred::AfterSysPropsInit => {
            static HOOKED_PROP_INIT: LazyLock<Result<(), eyre::Error>> =
                LazyLock::new(hook_sys_prop_init);

            HOOKED_PROP_INIT.as_ref().map_err(|e| eyre!(e))?;

            &AFTER_SYS_PROPS_INIT
        }
        Deferred::AfterDbgPropsInit => {
            static HOOKED_PROP_INIT: LazyLock<Result<(), eyre::Error>> =
                LazyLock::new(hook_dbg_prop_init);

            HOOKED_PROP_INIT.as_ref().map_err(|e| eyre!(e))?;

            &AFTER_DBG_PROPS_INIT
        }
    };

    deferred
        .lock()
        .unwrap()
        .as_mut()
        .map(|deferred| deferred.push(Box::new(move || span.in_scope(f))))
        .ok_or_eyre("tried to defer function after init")
}

#[instrument(ret(level = Level::DEBUG))]
fn hook_steam_init() -> Result<(), eyre::Error> {
    unsafe {
        let exe = Executable::new();
        let installer = install_iat!(exe, "steam_api64.dll", SteamAPI_Init = fn() -> bool)?;

        installer.static_hook_once(|hook| {
            || {
                let res = hook.call_original(());

                if res && let Some(deferred) = AFTER_MAIN.lock().unwrap().take() {
                    deferred.into_iter().for_each(|f| f());
                }

                res
            }
        });

        Ok(())
    }
}

#[instrument]
fn schedule_after_arxan() {
    let deferred = || {
        if let Some(deferred) = BEFORE_MAIN.lock().unwrap().take() {
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

fn hook_sys_prop_init() -> eyre::Result<()> {
    hook_prop_init("Core.System.DLPanic.Mode", || {
        if let Some(deferred) = AFTER_SYS_PROPS_INIT.lock().unwrap().take() {
            deferred.into_iter().for_each(|f| f());
        }
    })
}

fn hook_dbg_prop_init() -> eyre::Result<()> {
    hook_prop_init("Game.Debug.NearOnlyDraw", || {
        if let Some(deferred) = AFTER_DBG_PROPS_INIT.lock().unwrap().take() {
            deferred.into_iter().for_each(|f| f());
        }
    })
}

#[instrument(skip(hook))]
fn hook_prop_init<F>(property: &str, hook: F) -> eyre::Result<()>
where
    F: FnOnce() + Send + Sync + 'static,
{
    let exe = unsafe { Executable::new() };
    let hook_addr = find_prop_hook_addr(exe, property)?;

    unsafe {
        install_custom(hook_addr as *const ())
            .wrap_err("prop init hook creation failed")?
            .hook_once(|_| hook);
    }

    tracing::debug!("Game property map init hook installed");

    Ok(())
}

#[instrument(skip_all)]
fn find_prop_hook_addr<'a, P: Pe<'a>>(program: P, property: &str) -> eyre::Result<Va> {
    let rdata = program
        .section_headers()
        .by_name(".rdata")
        .wrap_err(".rdata section not found")?;
    let rdata_bytes = program.get_section_bytes(rdata)?;

    let property_wcstr: Vec<_> = property
        .encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect();

    let property_string_rva = rdata.VirtualAddress
        + memchr::memmem::find(rdata_bytes, &property_wcstr)
            .wrap_err_with(|| format!("Failed to find {property} in game executable"))?
            as Rva;

    let lea_rva = *me3_binary_analysis::util::lea_refs(program, property_string_rva)?
        .first()
        .wrap_err_with(|| format!("Failed to find LEA referencing {property}"))?;

    tracing::debug!("{property} lea RVA: {lea_rva:x}");

    let lea_va = program.rva_to_va(lea_rva)?;
    Ok(lea_va)
}
