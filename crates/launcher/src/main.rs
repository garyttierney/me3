#![windows_subsystem = "windows"]

use std::path::PathBuf;

use clap::{command, Parser};
use tracing::info;

use crate::game::Game;

mod game;

pub type LauncherResult<T> = eyre::Result<T>;

/// Launch a Steam game with the me3 mod loader attached.
#[derive(Parser, Debug)]
#[command(version)]
struct LauncherArgs {
    /// Path to the game EXE that should be launched.
    #[arg(long, env("ME3_GAME_EXE"))]
    exe: PathBuf,

    /// Path to the me3 that should be attached to the game.
    #[arg(long, env("ME3_DLL"))]
    dll: PathBuf,

    /// A list of paths to ModProfile configuration files.
    #[arg(short, long, action = clap::ArgAction::Append)]
    profiles: Vec<PathBuf>,
}

fn run() -> LauncherResult<()> {
    let args = LauncherArgs::parse();

    info!("Starting game at {:?} with DLL {:?}", args.exe, args.dll);

    let game_path = args.exe.parent();
    let mut game = Game::launch(&args.exe, game_path)?;

    if let Err(e) = game.attach(&args.dll) {
        println!("Attach error: {e:?}");
    }

    game.join();

    Ok(())
}

fn install_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();
}

fn install_panic_hook() {
    let _ = color_eyre::config::HookBuilder::default()
        .issue_url(concat!(env!("CARGO_PKG_REPOSITORY"), "/issues/new"))
        .add_issue_metadata("version", env!("CARGO_PKG_VERSION"))
        .issue_filter(|kind| match kind {
            color_eyre::ErrorKind::NonRecoverable(_) => false,
            color_eyre::ErrorKind::Recoverable(_) => true,
        })
        .install();
}

fn main() {
    install_tracing();
    install_panic_hook();

    run().expect("Failed to successfully run launcher");
}
