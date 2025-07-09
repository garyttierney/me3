#![feature(fn_traits)]
#![feature(fn_ptr_trait)]
#![feature(tuple_trait)]
#![feature(unboxed_closures)]

use std::{
    fs::OpenOptions,
    io::stdout,
    sync::{Arc, OnceLock},
};

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
mod filesystem;
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
        config:
            AttachConfig {
                game,
                natives,
                packages,
                ..
            },
    } = request;

    let telemetry_vars: TelemetryVars = me3_env::deserialize_from_env()?;
    let telemetry_log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&telemetry_vars.log_file_path)?;

    let telemetry_config = TelemetryConfig::default()
        .enabled(telemetry_vars.enabled)
        .with_console_writer(stdout)
        .with_file_writer(telemetry_log_file)
        .capture_panics(true);

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

        filesystem::attach_override(override_mapping.clone())?;

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
