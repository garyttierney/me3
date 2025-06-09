#![feature(fn_traits)]
#![feature(fn_ptr_trait)]
#![feature(tuple_trait)]
#![feature(unboxed_closures)]
#![feature(mapped_lock_guards)]

use std::{
    mem,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use crash_handler::CrashEventResult;
use eyre::Context;
use ipc_channel::ipc::IpcSender;
use me3_env::TelemetryVars;
use me3_launcher_attach_protocol::{
    AttachConfig, AttachRequest, AttachResult, Attachment, HostMessage,
};
use me3_mod_host_assets::mapping::ArchiveOverrideMapping;
use me3_telemetry::TelemetryConfig;
use tracing::info;

use crate::host::{hook::thunk::ThunkPool, ModHost};

mod asset_hooks;
mod detour;
mod host;

static INSTANCE: OnceLock<usize> = OnceLock::new();
static mut TELEMETRY_INSTANCE: OnceLock<me3_telemetry::Telemetry> = OnceLock::new();

/// https://learn.microsoft.com/en-us/windows/win32/dlls/dllmain#parameters
const DLL_PROCESS_DETACH: u32 = 0;
const DLL_PROCESS_ATTACH: u32 = 1;

dll_syringe::payload_procedure! {
    fn me_attach(request: AttachRequest) -> AttachResult {
        on_attach(request)
    }
}

fn on_attach(request: AttachRequest) -> AttachResult {
    me3_telemetry::install_error_handler();

    let AttachRequest {
        monitor_name,
        config:
            AttachConfig {
                game,
                natives,
                packages,
            },
    } = request;

    let socket = IpcSender::connect(monitor_name).unwrap();
    let socket = Arc::new(Mutex::new(socket));
    let crash_handler_socket = socket.clone();

    let crash_handler_guard = crash_handler::CrashHandler::attach(unsafe {
        crash_handler::make_crash_event(move |crash_context: &crash_handler::CrashContext| {
            info!("Handling crash event");
            let _ = crash_handler_socket
                .lock()
                .unwrap()
                .send(HostMessage::CrashDumpRequest {
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

    socket.lock().unwrap().send(HostMessage::Attached).unwrap();

    let telemetry_config = me3_env::deserialize_from_env()
        .wrap_err("couldn't deserialize env vars")
        .and_then(|vars: TelemetryVars| TelemetryConfig::try_from(vars))
        .expect("couldn't get telemetry config");

    let telemetry_guard = me3_telemetry::install(telemetry_config);

    #[allow(static_mut_refs)]
    let _ = unsafe { TELEMETRY_INSTANCE.set(telemetry_guard) };

    let result = me3_telemetry::with_root_span("host", "attach", move || {
        info!("Beginning host attach");

        let mut host = ModHost::new(ThunkPool::new()?);

        for native in natives {
            host.load_native(&native.path, native.initializer)?;
        }

        host.attach();
        let mut override_mapping = ArchiveOverrideMapping::new()?;
        override_mapping.scan_directories(packages.iter())?;
        let override_mapping = Arc::new(override_mapping);

        info!("Host successfully attached");

        asset_hooks::attach_override(game, override_mapping.clone())?;

        info!("Applied asset override hooks");

        Ok(Attachment)
    })?;

    Ok(result)
}

#[no_mangle]
pub extern "stdcall" fn DllMain(instance: usize, reason: u32, _: *mut usize) -> i32 {
    match reason {
        DLL_PROCESS_ATTACH => {
            let _ = INSTANCE.set(instance);
        }
        DLL_PROCESS_DETACH => {
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
