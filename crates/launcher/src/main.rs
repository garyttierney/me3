use std::path::PathBuf;

use clap::{command, Parser};
use me3_launcher_attach_protocol::AttachRequest;
use me3_mod_protocol::ModProfile;
use tracing::info;

use crate::game::Game;

mod game;

pub type LauncherResult<T> = color_eyre::Result<T>;

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
    info!("Launcher started");

    let args = match LauncherArgs::try_parse() {
        Ok(args) => args,
        Err(e) => e.exit(),
    };

    if args.profiles.is_empty() {
        info!("No profiles provided");
    } else {
        info!("Loading profiles from {:?}", args.profiles);
    }

    let profiles: Vec<_> = args
        .profiles
        .iter()
        .map(|path| ModProfile::from_file(path))
        .collect::<Result<_, _>>()?;

    let mut request = AttachRequest::default();
    for profile in profiles {
        request.profiles.push(profile)
    }

    info!("Starting game at {:?} with DLL {:?}", args.exe, args.dll);

    let game_path = args.exe.parent();
    let mut game = Game::launch(&args.exe, game_path)?;

    if let Err(e) = game.attach(&args.dll, request) {
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
        .install();
}

fn main() {
    println!("test");

    install_tracing();
    install_panic_hook();

    run().expect("Failed to successfully run launcher");
}
