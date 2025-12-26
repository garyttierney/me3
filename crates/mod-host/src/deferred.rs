use std::{
    ffi::c_int,
    iter,
    sync::{Mutex, Once, OnceLock},
};

use eyre::{eyre, OptionExt};
use pelite::{
    image::{IMAGE_DIRECTORY_ENTRY_IMPORT, IMAGE_IMPORT_DESCRIPTOR},
    pe::{image::IMAGE_ORDINAL_FLAG, Pe, PeObject},
};
use tracing::{debug, info, instrument, Span};
use windows::core::{PCWSTR, PWSTR};

use crate::{executable::Executable, host::ModHost};

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
pub fn defer_init<F>(span: Span, exe: Executable, until: Deferred, f: F) -> Result<(), eyre::Error>
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
            static HOOKED: OnceLock<Result<(), eyre::Error>> = OnceLock::new();

            HOOKED
                .get_or_init(|| hook_cmd_to_argv(exe))
                .as_ref()
                .map_err(|e| eyre!(e))?;

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

#[instrument(skip_all)]
fn hook_cmd_to_argv(exe: Executable) -> Result<(), eyre::Error> {
    type CmdToArgvW = Option<unsafe extern "C" fn(PCWSTR, *mut c_int) -> PWSTR>;

    static mut CMD_TO_ARGV_ORIGINAL: CmdToArgvW = None;

    unsafe extern "C" fn hook(cmd_line: PCWSTR, num_args: *mut c_int) -> PWSTR {
        if let Some(deferred) = DEFERRED_AFTER_MAIN.lock().unwrap().take() {
            deferred.into_iter().for_each(|f| f());
        }
        unsafe { CMD_TO_ARGV_ORIGINAL.unwrap()(cmd_line, num_args) }
    }

    let imports_dir = exe.data_directory()[IMAGE_DIRECTORY_ENTRY_IMPORT];
    debug!(?imports_dir);

    let mut imports = (0..imports_dir.Size)
        .step_by(size_of::<IMAGE_IMPORT_DESCRIPTOR>())
        .map(|pos| exe.derva_copy::<IMAGE_IMPORT_DESCRIPTOR>(imports_dir.VirtualAddress + pos));

    let shell32_imports = imports
        .find(|desc| {
            desc.and_then(|desc| exe.derva_c_str(desc.Name))
                .is_ok_and(|name| name.to_ascii_lowercase() == b"shell32.dll")
        })
        .ok_or_eyre("shell32.dll import table not found")??;

    debug!(?shell32_imports);

    let mut int = iter::from_fn({
        let mut pos = shell32_imports.OriginalFirstThunk;
        move || {
            let entry = exe.derva_copy::<u64>(pos).ok()?;
            pos += size_of::<u64>() as u32;
            Some(entry)
        }
    });

    let cmd_to_argv_hint = int
        .find_map(|entry| {
            if entry & IMAGE_ORDINAL_FLAG == 0 {
                let hint = exe.derva_copy::<u16>(entry as u32).ok()?;
                let name = exe.derva_c_str(entry as u32 + 2).ok()?;
                (name == b"CommandLineToArgvW").then_some(hint)
            } else {
                None
            }
        })
        .ok_or_eyre("CommandLineToArgvW not found in import table")?;

    debug!(?cmd_to_argv_hint);

    let cmd_to_argv = unsafe {
        exe.image().as_ptr().byte_add(
            shell32_imports.FirstThunk as usize + cmd_to_argv_hint as usize * size_of::<u64>(),
        ) as *mut CmdToArgvW
    };

    debug!("CommandLineToArgvW" = ?unsafe{ cmd_to_argv.read_unaligned() });

    unsafe {
        CMD_TO_ARGV_ORIGINAL = cmd_to_argv.read_unaligned();
        cmd_to_argv.write_unaligned(Some(hook));
    };

    Ok(())
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
