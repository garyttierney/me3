#![windows_subsystem = "windows"]
#![feature(windows_process_extensions_main_thread_handle)]

use std::fs::OpenOptions;

use me3_env::{LauncherVars, TelemetryVars};
use me3_launcher_attach_protocol::{AttachConfig, AttachRequest};
use me3_telemetry::TelemetryConfig;
use tracing::{info, instrument, warn};

use crate::{game::Game, steam::require_steam, writer::MakeWriterWrapper};

mod game;
mod steam;
mod writer;

pub type LauncherResult<T> = eyre::Result<T>;

#[instrument(skip_all)]
fn run(
    console_log_writer: MakeWriterWrapper,
    file_log_writer: MakeWriterWrapper,
) -> LauncherResult<()> {
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

    match game.attach(&args.host_dll, console_log_writer, file_log_writer, request) {
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

    let telemetry_vars = me3_env::deserialize_from_env::<TelemetryVars>()?;

    let monitor_pipe = OpenOptions::new()
        .read(false)
        .write(true)
        .open(&telemetry_vars.monitor_pipe_path)?;

    let console_log_writer = MakeWriterWrapper::new(monitor_pipe);

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&telemetry_vars.log_file_path)?;

    let file_log_writer = MakeWriterWrapper::new(log_file);

    let telemetry_config = TelemetryConfig::default()
        .enabled(telemetry_vars.enabled)
        .with_console_writer(console_log_writer.clone())
        .with_file_writer(file_log_writer.clone())
        .capture_panics(true);

    let _telemetry = me3_telemetry::install(telemetry_config);

    me3_telemetry::with_root_span("launcher", "run", move || {
        run(console_log_writer, file_log_writer)
    })
}
