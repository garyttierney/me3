#![feature(fn_traits)]
#![feature(fn_ptr_trait)]
#![feature(tuple_trait)]
#![feature(unboxed_closures)]
#![feature(mapped_lock_guards)]

use std::{
    io::PipeWriter,
    mem,
    os::windows::{prelude::FromRawHandle, raw::HANDLE},
    sync::{mpsc::RecvTimeoutError, Arc, OnceLock},
    time::Duration,
};

use crash_handler::CrashEventResult;
use eyre::Context;
use me3_env::TelemetryVars;
use me3_launcher_attach_protocol::{
    AttachConfig, AttachRequest, AttachResult, Attachment, HostMessage,
};
use me3_mod_host_assets::mapping::ArchiveOverrideMapping;
use me3_telemetry::TelemetryConfig;
use tracing::{error, info, Span};

use crate::{debugger::suspend_for_debugger, deferred::defer_until_init, host::ModHost};

mod asset_hooks;
mod debugger;
mod deferred;
mod detour;
mod host;
mod native;

static INSTANCE: OnceLock<usize> = OnceLock::new();
static mut TELEMETRY_INSTANCE: OnceLock<me3_telemetry::Telemetry> = OnceLock::new();

/// https://learn.microsoft.com/en-us/windows/win32/dlls/dllmain#parameters
const DLL_PROCESS_DETACH: u32 = 0;
const DLL_PROCESS_ATTACH: u32 = 1;

dll_syringe::payload_procedure! {
    fn me_attach(request: AttachRequest) -> AttachResult {
        if request.config.suspend {
            suspend_for_debugger();
        }

        on_attach(request)
    }
}

#[cfg(coverage)]
#[unsafe(no_mangle)]
#[allow(non_upper_case_globals)]
static __lvm_profile_runtime: i32 = 1;

#[cfg(coverage)]
unsafe extern "C" {
    fn __llvm_profile_write_file() -> i32;
    fn __llvm_profile_initialize_file();
}

fn on_attach(request: AttachRequest) -> AttachResult {
    me3_telemetry::install_error_handler();

    let AttachRequest {
        monitor_handle,
        config:
            AttachConfig {
                game,
                natives,
                packages,
                ..
            },
    } = request;

    let monitor_handle = HANDLE::from(monitor_handle as *mut _);
    let mut monitor_pipe = unsafe { PipeWriter::from_raw_handle(monitor_handle) };
    let (monitor_tx, monitor_rx) = std::sync::mpsc::channel::<HostMessage>();

    std::thread::spawn(move || loop {
        match monitor_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(msg) => {
                if msg.write_to(&mut monitor_pipe).is_err() {
                    break;
                }
            }
            Err(RecvTimeoutError::Timeout) => continue,
            Err(_) => break,
        }
    });

    let crash_handler_guard = crash_handler::CrashHandler::attach(unsafe {
        let monitor_tx = monitor_tx.clone();

        crash_handler::make_crash_event(move |crash_context: &crash_handler::CrashContext| {
            info!("Handling crash event");
            let _ = monitor_tx.send(HostMessage::CrashDumpRequest {
                exception_pointers: crash_context.exception_pointers as u64,
                process_id: crash_context.process_id,
                thread_id: crash_context.thread_id,
                exception_code: crash_context.exception_code,
            });

            // TODO: we need a safe way keep the process alive until the minidump is captured.
            std::thread::sleep(Duration::from_secs(5));

            CrashEventResult::Handled(false)
        })
    })?;

    // Keep the crash handler installed for the duration of the program.
    // `CrashHandler` is an empty struct with a `Drop` impl that uninstalls
    // the program-wide handler.
    mem::forget(crash_handler_guard);

    let _ = monitor_tx.send(HostMessage::Attached);

    let telemetry_config = me3_env::deserialize_from_env()
        .wrap_err("couldn't deserialize env vars")
        .and_then(|vars: TelemetryVars| TelemetryConfig::try_from(vars))
        .expect("couldn't get telemetry config");

    let telemetry_guard = me3_telemetry::install(telemetry_config);

    #[allow(static_mut_refs)]
    let _ = unsafe { TELEMETRY_INSTANCE.set(telemetry_guard) };

    let result = me3_telemetry::with_root_span("host", "attach", move || {
        info!("Beginning host attach");

        let host = ModHost::new();

        host.attach();
        let mut override_mapping = ArchiveOverrideMapping::new()?;
        override_mapping.scan_directories(packages.iter())?;
        let override_mapping = Arc::new(override_mapping);

        info!("Host successfully attached");

        defer_until_init({
            let current_span = Span::current();
            let override_mapping = override_mapping.clone();

            move || {
                let _span_guard = current_span.enter();

                for native in natives {
                    let mut host = ModHost::get_attached_mut();
                    if let Err(e) = host.load_native(&native.path, native.initializer) {
                        error!(
                            error = &*e,
                            "failed to load native mod from {:?}", &native.path
                        )
                    }
                }

                if let Err(e) = asset_hooks::attach_override(game, override_mapping) {
                    error!(
                        "error" = &*e,
                        "failed to attach asset override hooks; no files will be overridden"
                    )
                }
            }
        })?;

        info!("Deferred asset override hooks");

        Ok(Attachment)
    })?;

    Ok(result)
}

#[unsafe(no_mangle)]
pub extern "system" fn DllMain(instance: usize, reason: u32, _: *mut usize) -> i32 {
    match reason {
        DLL_PROCESS_ATTACH => {
            #[cfg(coverage)]
            unsafe {
                __llvm_profile_initialize_file()
            };

            let _ = INSTANCE.set(instance);
        }
        DLL_PROCESS_DETACH => {
            #[cfg(coverage)]
            unsafe {
                __llvm_profile_write_file()
            };

            std::thread::spawn(|| {
                #[allow(static_mut_refs)]
                let telemetry = unsafe { TELEMETRY_INSTANCE.take() };
                drop(telemetry);
            });
        }
        _ => {}
    }

    1
}
