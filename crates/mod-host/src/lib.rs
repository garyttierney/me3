#![feature(fn_traits)]
#![feature(fn_ptr_trait)]
#![feature(tuple_trait)]
#![feature(unboxed_closures)]

use std::sync::{Arc, Mutex, OnceLock};

use eyre::OptionExt;
use me3_binary_analysis::{fd4_step::Fd4StepTables, rtti};
use me3_env::TelemetryVars;
use me3_ipc::{
    bridge::BridgeToParent,
    message::MsgToChild,
    request::{Request, RequestId},
};
use me3_launcher_attach_protocol::{AttachConfig, AttachRequest, AttachResult, Attachment};
use me3_mod_host_assets::mapping::VfsOverrideMapping;
use me3_telemetry::TelemetryConfig;
use tracing::{error, info, instrument, warn, Span};
use windows::Win32::{
    Globalization::CP_UTF8,
    System::{
        Console::SetConsoleOutputCP,
        SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
    },
};

use crate::{
    deferred::{defer_init, Deferred},
    executable::Executable,
    host::{game_properties, ModHost},
};

mod alloc_hooks;
mod asset_hooks;
mod debugger;
mod deferred;
mod detour;
mod executable;
mod filesystem;
mod host;
mod native;
mod savefile;
mod skip_logos;

static INSTANCE: OnceLock<usize> = OnceLock::new();
static mut TELEMETRY_INSTANCE: OnceLock<me3_telemetry::Telemetry> = OnceLock::new();

fn me_attach(request: AttachRequest) -> AttachResult {
    if request.config.suspend {
        debugger::suspend_for_debugger();
    }

    on_attach(request)
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

    let bridge = me3_ipc::bridge::to_parent()?;

    let console_writer = bridge.console_log_writer();
    let file_writer = bridge.file_log_writer();

    let telemetry_config = TelemetryConfig::default()
        .enabled(telemetry_vars.enabled)
        .with_console_writer(move || console_writer.clone())
        .with_file_writer(move || file_writer.clone())
        .capture_panics(true);

    let telemetry_guard = me3_telemetry::install(telemetry_config);

    #[allow(static_mut_refs)]
    let _ = unsafe { TELEMETRY_INSTANCE.set(telemetry_guard) };

    let result = me3_telemetry::with_root_span("host", "attach", move || {
        info!("Beginning host attach");

        // SAFETY: process is still suspended.
        let exe = unsafe { Executable::new() };

        match exe.version() {
            Ok(ver) => info!("Attaching to {ver}"),
            Err(e) => warn!("error" = %e, "could not detect game version"),
        }

        ModHost::new(&attach_config).attach();

        dearxan(&attach_config)?;

        skip_logos::attach_override(attach_config.clone(), exe)?;

        game_properties::attach_override(attach_config.clone(), exe)?;

        if !attach_config.start_online {
            game_properties::start_offline();
        }

        let mut override_mapping = VfsOverrideMapping::new()?;
        override_mapping.scan_directories(attach_config.packages.iter())?;
        savefile::attach_override(&attach_config, &mut override_mapping)?;

        let override_mapping = Arc::new(override_mapping);

        filesystem::attach_override(override_mapping.clone())?;

        info!("Host successfully attached");

        let before_main_result = Arc::new(Mutex::new(None));

        defer_init(Span::current(), Deferred::BeforeMain, {
            let result = before_main_result.clone();
            let attach_config = attach_config.clone();
            move || *result.lock().unwrap() = Some(before_game_main(attach_config, exe))
        })?;

        defer_init(Span::current(), Deferred::AfterMain, move || {
            let result = after_game_main(attach_config, exe, override_mapping, move || {
                before_main_result
                    .lock()
                    .unwrap()
                    .take()
                    .ok_or_eyre("`before_game_main` did not run?")?
            });

            if let Err(e) = result {
                error!("error" = &*e, "deferred attach failed!")
            }
        })?;

        info!("Deferred me3 attach");

        Ok(Attachment)
    })?;

    Ok(result)
}

fn before_game_main(attach_config: Arc<AttachConfig>, exe: Executable) -> Result<(), eyre::Error> {
    if attach_config.mem_patch {
        alloc_hooks::hook_system_allocator(&attach_config, exe)?;
    }

    for native in &attach_config.early_natives {
        ModHost::get_attached().load_native(&native.path, &native.initializer)?;
    }

    Ok(())
}

fn after_game_main<R: FnOnce() -> Result<(), eyre::Error>>(
    attach_config: Arc<AttachConfig>,
    exe: Executable,
    override_mapping: Arc<VfsOverrideMapping>,
    before_main_result: R,
) -> Result<(), eyre::Error> {
    before_main_result()?;

    let class_map = Arc::new(rtti::classes(exe)?);
    let step_tables = Fd4StepTables::from_initialized_data(exe)?;

    if attach_config.mem_patch {
        alloc_hooks::hook_heap_allocators(&attach_config, exe, &class_map)?;
    }

    savefile::oversized_regulation_fix(
        attach_config.clone(),
        exe,
        &step_tables,
        override_mapping.clone(),
    )?;

    let first_delayed_offset = attach_config
        .natives
        .iter()
        .enumerate()
        .filter_map(|(idx, native)| native.initializer.is_some().then_some(idx))
        .next()
        .unwrap_or(attach_config.natives.len());

    let (immediate, delayed) = attach_config.natives.split_at(first_delayed_offset);

    for native in immediate {
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

    let delayed = delayed.to_vec();
    std::thread::spawn(move || {
        for native in delayed {
            if let Err(e) = ModHost::get_attached().load_native(&native.path, &native.initializer) {
                warn!(
                    error = &*e,
                    path = %native.path.display(),
                    "failed to load native mod",
                );

                if !native.optional {
                    panic!("{:#?}", e);
                }
            }
        }
    });

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

fn dearxan(attach_config: &AttachConfig) -> Result<(), eyre::Error> {
    if !ModHost::get_attached().disable_arxan {
        return Ok(());
    }

    info!(
        "game" = %attach_config.game,
        "attach_config.disable_arxan" = attach_config.disable_arxan,
        "will attempt to disable Arxan code protection",
    );

    defer_init(Span::current(), Deferred::BeforeMain, || {
        info!("dearxan::disabler::neuter_arxan finished")
    })
}

fn spawn_msg_thread() {
    std::thread::spawn(|| {
        let bridge = me3_ipc::bridge::to_parent().unwrap();
        let recv_span = bridge.enter_recv_span().unwrap();

        loop {
            let msg = match recv_span.recv() {
                Ok(msg) => msg,
                Err(error) => {
                    error!(%error, "failed to receive message");
                    continue;
                }
            };

            match msg {
                MsgToChild::Request(req) => fulfill_request(&bridge, req),
            }
        }
    });
}

#[instrument(skip_all)]
fn fulfill_request(bridge: &BridgeToParent, (id, req): (RequestId, Request)) {
    let result = match req {
        Request::Attach(_) => bridge.fulfill((id, req), me_attach),
    };

    if let Err(error) = result {
        error!(%error, "failed to fulfill request");
    }
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

            spawn_msg_thread();
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
