#![windows_subsystem = "windows"]
#![feature(windows_process_extensions_main_thread_handle)]

use std::{
    fs::{File, OpenOptions},
    sync::{Mutex, OnceLock},
};

use me3_env::{LauncherVars, TelemetryVars};
use me3_launcher_attach_protocol::{AttachConfig, AttachRequest};
use me3_telemetry::TelemetryConfig;
use tracing::{info, instrument, warn};
use tracing_subscriber::fmt::writer::MakeWriter;

use crate::{game::Game, steam::require_steam};

mod game;
mod steam;

pub type LauncherResult<T> = eyre::Result<T>;

static MONITOR_PIPE: OnceLock<Mutex<File>> = OnceLock::new();

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
        warn!("skipping Steam initialization, no guarantee Steam game will launch successfully");
    }

    let game_path = args.exe.parent();
    let mut game = Game::launch(&args.exe, game_path)?;
    let request = AttachRequest { config };

    match game.attach(&args.host_dll, request) {
        Ok(_) => info!("attached to game successfully"),
        Err(e) => {
            game.child.kill()?;
            return Err(e);
        }
    }

    game.join();

    Ok(())
}

fn main() -> LauncherResult<()> {
    me3_telemetry::install_error_handler();

    let mut telemetry_vars = me3_env::deserialize_from_env::<TelemetryVars>()?;

    if let Some(path) = telemetry_vars.monitor_pipe_path.take() {
        let pipe = OpenOptions::new().read(false).append(true).open(&path)?;
        MONITOR_PIPE.get_or_init(move || Mutex::new(pipe));
    }

    let mut telemetry_config = TelemetryConfig::try_from(telemetry_vars)?;

    if let Some(monitor_pipe) = MONITOR_PIPE.get() {
        telemetry_config = telemetry_config.with_console_writer(|| monitor_pipe.make_writer())
    }

    let _telemetry = me3_telemetry::install(telemetry_config);

    me3_telemetry::with_root_span("launcher", "run", run)
}
