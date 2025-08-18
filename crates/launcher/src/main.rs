#![windows_subsystem = "windows"]
#![feature(windows_process_extensions_main_thread_handle)]

use eyre::Context;
use me3_env::{LauncherVars, TelemetryVars};
use me3_launcher_attach_protocol::{AttachConfig, AttachRequest};
use me3_telemetry::TelemetryConfig;
use tracing::{error, info, instrument, warn};

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
        warn!("skipping Steam initialization, no guarantee Steam game will launch successfully");
    }

    let game_path = args.exe.parent();
    let game = Game::launch(&args.exe, game_path)?;
    let request = AttachRequest { config };

    match game.attach(&args.host_dll, request) {
        Ok(_) => info!("attached to game successfully"),
        Err(error) => {
            error!(
                error = &*error,
                "an error occurred while loading me3, modded content will not be available"
            );
        }
    }

    game.join();

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
