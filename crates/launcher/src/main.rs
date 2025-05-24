use std::{
    fs::File,
    io,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering::SeqCst},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use clap::{command, Parser};
use crash_context::CrashContext;
use eyre::OptionExt;
use ipc_channel::ipc::{IpcError, IpcOneShotServer};
use me3_launcher_attach_protocol::{AttachRequest, HostMessage};
use me3_mod_protocol::{dependency::sort_dependencies, ModProfile};
use minidump_writer::minidump_writer::MinidumpWriter;
use tracing::{error, info};

use crate::game::Game;

mod game;

pub type LauncherResult<T> = stable_eyre::Result<T>;

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

    let mut natives = vec![];
    let mut packages = vec![];

    // TODO: merge
    if let Some(path) = args.profiles.into_iter().next() {
        let base = path
            .parent()
            .ok_or_eyre("failed to acquire base directory for mod profile")?;

        let mut profile = ModProfile::from_file(&path)?;
        let mut profile_packages = profile.packages();

        // Make relative paths absolute
        profile_packages
            .iter_mut()
            .filter(|e| e.is_relative())
            .for_each(|e| e.make_absolute(base));

        let ordered_natives = sort_dependencies(profile.natives())
            .ok_or_eyre("failed to create dependency graph for natives")?;

        let ordered_packages = sort_dependencies(profile.packages())
            .ok_or_eyre("failed to create dependency graph for packages")?;

        natives.extend(ordered_natives);
        packages.extend(ordered_packages);
    }

    let (monitor_server, monitor_name) = IpcOneShotServer::new()?;
    let request = AttachRequest {
        monitor_name,
        natives,
        packages,
    };

    info!("Starting game at {:?} with DLL {:?}", args.exe, args.dll);

    let shutdown_requested = Arc::new(AtomicBool::new(false));

    let game_path = args.exe.parent();
    let mut game = Game::launch(&args.exe, game_path)?;

    let monitor_thread = std::thread::spawn(move || {
        info!("Starting monitor thread");
        let (receiver, client) = monitor_server.accept().unwrap();
        info!("Host connected to monitor with message {:?}", client);

        loop {
            match receiver.recv() {
                Ok(msg) => match msg {
                    HostMessage::Trace(message) => {
                        info!(message);
                    }
                    HostMessage::CrashDumpRequest {
                        exception_pointers,
                        process_id,
                        thread_id,
                        exception_code,
                    } => {
                        info!(
                            "Host requested a crashdump for exception {:x}",
                            exception_code
                        );

                        let start = SystemTime::now();
                        let timestamp = start
                            .duration_since(UNIX_EPOCH)
                            .expect("system clock is broken")
                            .as_secs();

                        let mut file = File::create_new(format!("me3_crash_{timestamp}.dmp"))
                            .expect("unable to create crash dump file");

                        MinidumpWriter::dump_crash_context(
                            CrashContext {
                                exception_pointers: exception_pointers as *const _,
                                exception_code,
                                process_id,
                                thread_id,
                            },
                            None,
                            &mut file,
                        )
                        .expect("faild to write crash dump to file");
                    }
                    HostMessage::Attached => info!("Attach completed"),
                },
                Err(IpcError::Disconnected) => break,
                Err(e) => {
                    error!("Error from monitor channel {:?}", e);
                    break;
                }
            }
        }
    });

    if let Err(e) = game.attach(&args.dll, request) {
        println!("Failed to attach to game: {e:?}");
    }

    game.join();
    shutdown_requested.store(true, SeqCst);
    let _ = monitor_thread.join();

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

fn main() {
    install_tracing();

    run().expect("Failed to successfully run launcher");
}
