#![windows_subsystem = "windows"]

use std::{
    env,
    fs::OpenOptions,
    os::windows::prelude::{AsHandle, AsRawHandle, IntoRawHandle},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering::SeqCst},
        Arc,
    },
};

use eyre::Context;
use me3_launcher_attach_protocol::{AttachRequest, MonitorPipeHandle};
use tracing::{error, info, info_span};
use windows::Win32::{
    Foundation::{
        DuplicateHandle, DUPLICATE_CLOSE_SOURCE, DUPLICATE_HANDLE_OPTIONS, DUPLICATE_SAME_ACCESS,
        HANDLE,
    },
    System::Threading::GetCurrentProcess,
};

use crate::game::Game;

mod game;
mod monitor;

pub type LauncherResult<T> = stable_eyre::Result<T>;

/// Launch a Steam game with the me3 mod loader attached.
#[derive(Debug)]
struct LauncherArgs {
    /// Path to the game EXE that should be launched.
    exe: PathBuf,

    /// Path to the me3 that should be attached to the game.
    dll: PathBuf,

    /// Path to the compiled attach configuration.
    config_path: PathBuf,
}

impl LauncherArgs {
    pub fn from_env() -> Result<Self, eyre::Error> {
        let exe = PathBuf::from(env::var("ME3_GAME_EXE").wrap_err("game exe wasn't set")?);
        let dll = PathBuf::from(env::var("ME3_HOST_DLL").wrap_err("me3 host dll wasn't set")?);
        let config_path = PathBuf::from(
            env::var("ME3_HOST_CONFIG_PATH").wrap_err("me3 host config path wasn't set")?,
        );

        Ok(LauncherArgs {
            exe,
            dll,
            config_path,
        })
    }
}

fn run() -> LauncherResult<()> {
    let span = info_span!("run");
    let span_guard = span.enter();
    info!("Launcher started");

    let args = LauncherArgs::from_env()?;

    info!(?args, "Parsed launcher args");

    let attach_config_text = std::fs::read_to_string(args.config_path)?;
    let attach_config = toml::from_str(&attach_config_text)?;

    info!("Starting game at {:?} with DLL {:?}", args.exe, args.dll);

    let shutdown_requested = Arc::new(AtomicBool::new(false));

    let mut game = Game::launch(&args.exe, args.exe.parent())?;
    let mut game_monitor_handle = HANDLE::default();
    let game_pid = game.child.id() as u64;

    let (pipe_rx, pipe_tx) = std::io::pipe()?;

    unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            HANDLE(pipe_tx.into_raw_handle()),
            std::mem::transmute(game_pid),
            &raw mut game_monitor_handle,
            0,
            true,
            DUPLICATE_SAME_ACCESS | DUPLICATE_CLOSE_SOURCE,
        )?;
    }

    let monitor_thread_shutdown = shutdown_requested.clone();
    let monitor_thread = monitor::run_monitor(monitor_thread_shutdown, pipe_rx);

    let request = AttachRequest {
        monitor_pipe: MonitorPipeHandle(game_monitor_handle.0.addr()),
        config: attach_config,
    };

    if let Err(e) = game.attach(&args.dll, request) {
        error!("Failed to attach to game: {e:?}");
    }

    drop(span_guard);

    game.join();
    shutdown_requested.store(true, SeqCst);
    let _ = monitor_thread.join();

    Ok(())
}

fn main() {
    let log_file_path = std::env::var("ME3_LOG_FILE").expect("log file location not set");
    let log_file = OpenOptions::new()
        .append(true)
        .open(log_file_path)
        .expect("couldn't open log file");

    let _guard = me3_telemetry::install(std::env::var("ME3_TELEMETRY").is_ok(), move || {
        log_file.try_clone().unwrap()
    });

    if let Err(e) = run() {
        error!(?e, "Failed to run launcher: {e}");
    }
}
