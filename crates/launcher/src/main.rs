use std::{io, path::PathBuf};

use clap::{command, Parser};
use eyre::OptionExt;
use me3_launcher_attach_protocol::AttachRequest;
use me3_mod_protocol::{dependency::sort_dependencies, ModProfile};
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

    let mut natives = vec![];
    let mut packages = vec![];

    // TODO: merge
    if let Some(mut profile) = profiles.into_iter().next() {
        // let ordered_natives = sort_dependencies(profile.natives())
        //     .ok_or_eyre("failed to create dependency graph for natives")?;
        //
        // let ordered_packages = sort_dependencies(profile.packages())
        //     .ok_or_eyre("failed to create dependency graph for packages")?;
        //
        // natives.extend(ordered_natives);
        // packages.extend(ordered_packages);
        natives.extend(profile.natives());
        packages.extend(profile.packages());
    }

    let request = AttachRequest { natives, packages };

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

    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(tracing_appender::rolling::never(".", "me3.log"))
        .pretty();
    let stdout_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(io::stderr)
        .with_target(false)
        .compact();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(stdout_layer)
        .with(file_layer)
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
    install_tracing();
    install_panic_hook();

    run().expect("Failed to successfully run launcher");
}
