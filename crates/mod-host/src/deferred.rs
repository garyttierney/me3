use std::{
    mem,
    sync::{LazyLock, Mutex, Once},
};

use eyre::{eyre, Context, ContextCompat, OptionExt};
use pelite::pe::{Pe, Rva, Va};
use retour::RawDetour;
use tracing::{info, instrument, Level, Span};
use windows::{
    core::{s, w},
    Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW},
};

use crate::{executable::Executable, host::ModHost};

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
                LazyLock::new(SysPropInitHook::hook_prop_init);

            HOOKED_PROP_INIT.as_ref().map_err(|e| eyre!(e))?;

            &AFTER_SYS_PROPS_INIT
        }
        Deferred::AfterDbgPropsInit => {
            static HOOKED_PROP_INIT: LazyLock<Result<(), eyre::Error>> =
                LazyLock::new(DbgPropInitHook::hook_prop_init);

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

#[instrument]
fn hook_steam_init() -> Result<(), eyre::Error> {
    ModHost::get_attached()
        .hook(steam_init_fn()?)
        .with_closure(|trampoline| {
            let result = unsafe { trampoline() };

            if result && let Some(deferred) = AFTER_MAIN.lock().unwrap().take() {
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

trait PropInitHook {
    const FIRST_PROPERTY: &str;

    const PROP_INIT_HOOK: &'static Mutex<Option<RawDetour>>;
    const PROP_INIT_TRAMPOLINE: *mut *const ();

    const AFTER_PROPS_INIT: &'static Mutex<DeferredOnce>;

    unsafe extern "C" fn prop_init_hook_raw_trampoline();

    #[instrument]
    fn hook_prop_init() -> eyre::Result<()> {
        let exe = unsafe { Executable::new() };
        let hook_addr = Self::find_prop_hook_addr(exe)?;

        let hook = unsafe {
            RawDetour::new(
                hook_addr as *const (),
                Self::prop_init_hook_raw as *const (),
            )
            .wrap_err("prop init hook creation failed")?
        };

        unsafe {
            hook.enable().wrap_err("prop init hook enable failed")?;
            *Self::PROP_INIT_TRAMPOLINE = hook.trampoline();
        }
        tracing::debug!("Game property map init hook installed");

        let _ = Self::PROP_INIT_HOOK.lock().unwrap().insert(hook);
        Ok(())
    }

    #[instrument(skip_all)]
    fn find_prop_hook_addr<'a, P: Pe<'a>>(program: P) -> eyre::Result<Va> {
        let rdata = program
            .section_headers()
            .by_name(".rdata")
            .wrap_err(".rdata section not found")?;
        let rdata_bytes = program.get_section_bytes(rdata)?;

        let property_wcstr: Vec<_> = Self::FIRST_PROPERTY
            .encode_utf16()
            .chain([0])
            .flat_map(u16::to_le_bytes)
            .collect();

        let property_string_rva = rdata.VirtualAddress
            + memchr::memmem::find(rdata_bytes, &property_wcstr).wrap_err_with(|| {
                format!("Failed to find {} in game executable", Self::FIRST_PROPERTY)
            })? as Rva;

        let lea_rva = *me3_binary_analysis::util::lea_refs(program, property_string_rva)?
            .first()
            .wrap_err_with(|| format!("Failed to find LEA referencing {}", Self::FIRST_PROPERTY))?;

        tracing::debug!("{} lea RVA: {lea_rva:x}", Self::FIRST_PROPERTY);
        Ok(program.rva_to_va(lea_rva)?)
    }

    // Use win64 so because it has the most callee-saved registers, which means we don't
    // have to save as much using inline assembly.
    unsafe extern "win64" fn prop_init_hook() {
        if let Some(deferred) = Self::AFTER_PROPS_INIT.lock().unwrap().take() {
            deferred.into_iter().for_each(|f| f());
        }
        if let Some(hook) = Self::PROP_INIT_HOOK.lock().unwrap().as_ref() {
            let _ = unsafe { hook.disable() };
        }
    }

    #[unsafe(naked)]
    unsafe extern "C" fn prop_init_hook_raw() {
        // Simple x64 context save/restore for win64 volatile registers.
        std::arch::naked_asm!(
            // Move below the sysv64 stack red zone, in case we eventually want to
            // support non-Windows targets (e.g. emulated Bloodborne).
            // Do this with a LEA instead of ADD/SUB so EFLAGS is unchanged.
            "lea rsp, [rsp - 0x80]",
            // First save EFLAGS, RSP and align the stack.
            // To branchlessly align the stack, we need a scratch register. Use RAX.
            "pushfq",
            "push rax",
            "mov rax, rsp",
            "and rsp, -16",
            "push rax",
            // Push other volatile GPRs.
            "push rcx",
            "push rdx",
            "push r8",
            "push r9",
            "push r10",
            "push r11",
            // Push volatile XMM registers.
            // Sub an extra 8 bytes to re-align the stack to 16 bytes after the GPR save.
            "sub rsp, 0x68",
            "movaps [rsp + 0x00], xmm0",
            "movaps [rsp + 0x10], xmm1",
            "movaps [rsp + 0x20], xmm2",
            "movaps [rsp + 0x30], xmm3",
            "movaps [rsp + 0x40], xmm4",
            "movaps [rsp + 0x50], xmm5",
            // Allocate shadow stack space and call the hook routine.
            "sub rsp, 0x20",
            "call {hook}",
            "add rsp, 0x20",
            // Restore volatile XMM registers.
            "movaps xmm0, [rsp + 0x00]",
            "movaps xmm1, [rsp + 0x10]",
            "movaps xmm2, [rsp + 0x20]",
            "movaps xmm3, [rsp + 0x30]",
            "movaps xmm4, [rsp + 0x40]",
            "movaps xmm5, [rsp + 0x50]",
            "add rsp, 0x68",
            // Restore volatile GPRs.
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rdx",
            "pop rcx",
            // Restore original RSP, RAX and EFLAGS.
            "pop rsp",
            "pop rax",
            "popfq",
            // Move RSP back to the top of the sysv64 stack red zone.
            "lea rsp, [rsp + 0x80]",
            // Jump back to the original address
            "jmp {trampoline}",
            hook = sym Self::prop_init_hook,
            trampoline = sym Self::prop_init_hook_raw_trampoline
        );
    }
}

struct SysPropInitHook;

static SYS_PROP_INIT_HOOK: Mutex<Option<RawDetour>> = Mutex::new(None);
static mut SYS_PROP_INIT_TRAMPOLINE: *const () = std::ptr::null();

impl PropInitHook for SysPropInitHook {
    const FIRST_PROPERTY: &str = "Core.System.DLPanic.Mode";

    const PROP_INIT_HOOK: &'static Mutex<Option<RawDetour>> = &SYS_PROP_INIT_HOOK;
    const PROP_INIT_TRAMPOLINE: *mut *const () = &raw mut SYS_PROP_INIT_TRAMPOLINE;

    const AFTER_PROPS_INIT: &'static Mutex<DeferredOnce> = &AFTER_SYS_PROPS_INIT;

    #[unsafe(naked)]
    unsafe extern "C" fn prop_init_hook_raw_trampoline() {
        std::arch::naked_asm!(
            "jmp qword ptr [rip+{trampoline}]",
            trampoline = sym SYS_PROP_INIT_TRAMPOLINE,
        );
    }
}

struct DbgPropInitHook;

static DBG_PROP_INIT_HOOK: Mutex<Option<RawDetour>> = Mutex::new(None);
static mut DBG_PROP_INIT_TRAMPOLINE: *const () = std::ptr::null();

impl PropInitHook for DbgPropInitHook {
    const FIRST_PROPERTY: &str = "Game.Debug.NearOnlyDraw";

    const PROP_INIT_HOOK: &'static Mutex<Option<RawDetour>> = &DBG_PROP_INIT_HOOK;
    const PROP_INIT_TRAMPOLINE: *mut *const () = &raw mut DBG_PROP_INIT_TRAMPOLINE;

    const AFTER_PROPS_INIT: &'static Mutex<DeferredOnce> = &AFTER_DBG_PROPS_INIT;

    #[unsafe(naked)]
    unsafe extern "C" fn prop_init_hook_raw_trampoline() {
        std::arch::naked_asm!(
            "jmp qword ptr [rip+{trampoline}]",
            trampoline = sym DBG_PROP_INIT_TRAMPOLINE,
        );
    }
}
