#![windows_subsystem = "windows"]

use std::{
    io::ErrorKind,
    os::windows::prelude::{AsRawHandle, IntoRawHandle},
    sync::{
        atomic::{AtomicBool, Ordering::SeqCst},
        Arc,
    },
    time::Duration,
};

use eyre::Context;
use me3_env::{LauncherVars, TelemetryVars};
use me3_launcher_attach_protocol::{AttachConfig, AttachRequest, HostMessage};
use me3_telemetry::TelemetryConfig;
use tracing::{error, info, instrument, warn};
use windows::Win32::{
    Foundation::{DuplicateHandle, DUPLICATE_CLOSE_SOURCE, DUPLICATE_SAME_ACCESS, HANDLE},
    System::Threading::GetCurrentProcess,
};

use crate::{game::Game, steam::require_steam};

mod game;
mod steam;

pub type LauncherResult<T> = eyre::Result<T>;

#[instrument]
fn run() -> LauncherResult<()> {
    info!("Launcher started");

    let args: LauncherVars = me3_env::deserialize_from_env()?;

    info!(?args, "Parsed launcher args");

    let attach_config_text = std::fs::read_to_string(args.host_config_path)?;
    let config: AttachConfig = toml::from_str(&attach_config_text)?;

    info!(
        "Starting game at {:?} with DLL {:?}",
        args.exe, args.host_dll
    );

    if !config.skip_steam_init {
        require_steam(&args.exe)?;
    } else {
        warn!("skpping steam initialization, no guarantee Steam game will launch successfully");
    }

    let game_path = args.exe.parent();
    let game = Game::launch(&args.exe, game_path)?;
    let mut game_monitor_handle = HANDLE::default();

    let (mut pipe_rx, pipe_tx) = std::io::pipe()?;

    unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            HANDLE(pipe_tx.into_raw_handle()),
            HANDLE(game.child.as_raw_handle()),
            &raw mut game_monitor_handle,
            0,
            true,
            DUPLICATE_SAME_ACCESS | DUPLICATE_CLOSE_SOURCE,
        )?;
    }

    let request = AttachRequest {
        monitor_handle: game_monitor_handle.0.addr(),
        config,
    };

    let shutdown_requested = Arc::new(AtomicBool::new(false));

    let _ = std::thread::spawn({
        let shutdown_requested = shutdown_requested.clone();

        move || {
            while !shutdown_requested.load(SeqCst) {
                match HostMessage::read_from(&mut pipe_rx) {
                    Ok(msg) => {
                        info!(?msg);
                    }
                    Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                        std::thread::yield_now();
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    Err(e) => {
                        info!(?e, "monitor exiting");
                        break;
                    }
                }
            }
        }
    });

    match game.attach(&args.host_dll, request) {
        Ok(_) => info!("attached to game successfully"),
        Err(error) => {
            error!(
                error = &*error,
                "an error occurred while loading me3, modded content will not be available"
            );
            shutdown_requested.store(true, SeqCst);
        }
    }

    game.join();
    shutdown_requested.store(true, SeqCst);

    Ok(())
}

fn main() {
    me3_telemetry::install_error_handler();

    let telemetry_config = me3_env::deserialize_from_env()
        .wrap_err("couldn't deserialize env vars")
        .and_then(|vars: TelemetryVars| TelemetryConfig::try_from(vars))
        .expect("couldn't get telemetry config");
    let _telemetry = me3_telemetry::install(telemetry_config);

    let _ = me3_telemetry::with_root_span("launcher", "run", run);
}
