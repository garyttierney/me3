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
use me3_launcher_attach_protocol::{AttachRequest, AttachResult, Attachment};
use me3_telemetry::TelemetryConfig;
use tracing::{info, warn, Span};
use windows::Win32::{
    Globalization::CP_UTF8,
    System::{
        Console::SetConsoleOutputCP,
        SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
    },
};

use crate::{
    app::{ExternalResource, Me3App, PostStartup, PreStartup, Startup},
    deferred::defer_until_init,
    executable::Executable,
    host::ModHost,
    plugins::{
        natives::NativesPlugin, properties::GamePropertiesPlugin, save_file::SaveFilePlugin,
        skip_logos::SkipLogosPlugin, vfs::VfsPlugin,
    },
};

pub mod app;
mod asset_hooks;
mod debugger;
mod deferred;
mod detour;
mod executable;
mod filesystem;
mod host;
mod native;
pub mod plugins;

static INSTANCE: OnceLock<usize> = OnceLock::new();
static mut TELEMETRY_INSTANCE: OnceLock<me3_telemetry::Telemetry> = OnceLock::new();

dll_syringe::payload_procedure! {
    fn me_attach(request: AttachRequest) -> AttachResult {
        if request.config.suspend {
            debugger::suspend_for_debugger();
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

    if debugger::is_debugger_present()
        && let Err(e) = debugger::prevent_hiding_threads()
    {
        warn!("error" = &*e, "may fail to debug some threads");
    }

    // SAFETY: process is still suspended.
    let exe = unsafe { Executable::new() };
    let mut app = Me3App::new();

    app.insert_resource(ExternalResource(exe));
    app.insert_resource(ExternalResource(attach_config));

    app.register_system(PreStartup, host::dearxan);
    // app.register_system(
    //     Startup,
    //     (
    //         game_properties::attach_override,
    //         game_properties::start_offline.run_if(|| !start_online),
    //         filesystem::attach_override,
    //     ),
    // );

    app.register_plugin(GamePropertiesPlugin);
    app.register_plugin(NativesPlugin);
    app.register_plugin(SaveFilePlugin);
    app.register_plugin(SkipLogosPlugin);
    app.register_plugin(VfsPlugin);

    // TODO: could load systems from dylibs here

    app.run_schedule(PreStartup);
    app.run_schedule(Startup);

    let host = ModHost::new(app);
    host.attach();

    info!("Host successfully attached");

    defer_until_init(Span::current(), {
        move || {
            ModHost::with_app(|_, app| {
                app.run_schedule(PostStartup);
            });
        }
    })?;

    Ok(Attachment)
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

            // FIXME: this panics on process exit, either on thread creation or accessing a thread
            // local. Ideally, it should be called at an earlier point.
            //
            // The crash handler (when re-added) should call flush instead.
            //
            // std::thread::spawn(|| {
            //     #[allow(static_mut_refs)]
            //     let telemetry = unsafe { TELEMETRY_INSTANCE.take() };
            //     drop(telemetry);
            // });
        }
        _ => {}
    }

    1
}
