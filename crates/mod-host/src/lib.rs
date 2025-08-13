#![feature(fn_traits)]
#![feature(fn_ptr_trait)]
#![feature(tuple_trait)]
#![feature(unboxed_closures)]

use std::{
    fs::OpenOptions,
    io::stdout,
    sync::{Arc, OnceLock},
};

use me3_binary_analysis::{fd4_step::Fd4StepTables, rtti};
use me3_env::TelemetryVars;
use me3_launcher_attach_protocol::{AttachConfig, AttachRequest, AttachResult, Attachment};
use me3_mod_host_assets::mapping::ArchiveOverrideMapping;
use me3_telemetry::TelemetryConfig;
use tracing::{error, info, warn, Span};
use windows::Win32::{
    Globalization::CP_UTF8,
    System::{
        Console::SetConsoleOutputCP,
        SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
    },
};

use crate::{
    debugger::suspend_for_debugger,
    deferred::defer_until_init,
    executable::Executable,
    host::{game_properties, ModHost},
};

mod asset_hooks;
mod debugger;
mod deferred;
mod detour;
mod executable;
mod filesystem;
mod host;
mod native;
mod skip_logos;

static INSTANCE: OnceLock<usize> = OnceLock::new();
static mut TELEMETRY_INSTANCE: OnceLock<me3_telemetry::Telemetry> = OnceLock::new();

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
static __llvm_profile_runtime: i32 = 1;

#[cfg(coverage)]
unsafe extern "C" {
    fn __llvm_profile_write_file() -> i32;
    fn __llvm_profile_initialize_file();
}

fn on_attach(request: AttachRequest) -> AttachResult {
    let _ = unsafe { SetConsoleOutputCP(CP_UTF8) };
    me3_telemetry::install_error_handler();

    let attach_config = Arc::new(request.config);

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

        // SAFETY: process is still suspended.
        let exe = unsafe { Executable::new() };

        ModHost::new().attach();

        host::dearxan(&attach_config);

        skip_logos::attach_override(attach_config.clone(), exe)?;

        game_properties::attach_override(attach_config.clone(), exe)?;

        if !attach_config.start_online {
            game_properties::start_offline();
        }

        let mut override_mapping = ArchiveOverrideMapping::new()?;
        override_mapping.scan_directories(attach_config.packages.iter())?;
        let override_mapping = Arc::new(override_mapping);

        filesystem::attach_override(override_mapping.clone())?;

        info!("Host successfully attached");

        defer_until_init(Span::current(), {
            let override_mapping = override_mapping.clone();

            move || {
                if let Err(e) = deferred_attach(attach_config, exe, override_mapping) {
                    error!("error" = &*e, "deferred attach failed!")
                }
            }
        })?;

        info!("Deferred me3 attach");

        Ok(Attachment)
    })?;

    Ok(result)
}

fn deferred_attach(
    attach_config: Arc<AttachConfig>,
    exe: Executable,
    override_mapping: Arc<ArchiveOverrideMapping>,
) -> Result<(), eyre::Error> {
    let class_map = Arc::new(rtti::classes(exe)?);
    let step_tables = Fd4StepTables::from_initialized_data(exe)?;

    for native in &attach_config.natives {
        if let Err(e) = ModHost::get_attached().load_native(&native.path, &native.initializer) {
            warn!(
                error = &*e,
                path = %native.path.display(),
                "failed to load native mod",
            );

            if !native.optional {
                return Err(e);
            }
        }
    }

    asset_hooks::attach_override(
        attach_config,
        exe,
        class_map,
        &step_tables,
        override_mapping,
    )
    .map_err(|e| {
        e.wrap_err("failed to attach asset override hooks; no files will be overridden")
    })?;

    Ok(())
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
