use std::{
    fs::File,
    io,
    path::PathBuf,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering::SeqCst},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use clap::{ArgAction, Parser};
use crash_context::CrashContext;
use eyre::OptionExt;
use ipc_channel::ipc::{IpcError, IpcOneShotServer};
use me3_launcher_attach_protocol::{AttachRequest, HostMessage};
use me3_mod_protocol::{dependency::sort_dependencies, package::WithPackageSource, ModProfile};
use minidump_writer::minidump_writer::MinidumpWriter;
use normpath::PathExt;
use sentry::types::Dsn;
#[cfg(feature = "sentry")]
use sentry::{
    protocol::{Attachment, AttachmentType, Event},
    Level,
};
use tracing::{debug, error, info, trace, warn};

use crate::game::Game;

mod game;

pub type LauncherResult<T> = stable_eyre::Result<T>;

/// Launch a Steam game with the me3 mod loader attached.
#[derive(Parser, Debug)]
#[command(version, disable_help_flag(true))]
struct LauncherArgs {
    /// Opt-in to the unstable command-line interface. The options and structure
    /// of the launcher CLI is currently unstable and may change in a later version.
    #[arg(long, action(ArgAction::SetTrue), required(false))]
    enable_unstable_cli: bool,

    /// Path to the game EXE that should be launched.
    #[arg(
        long,
        env("ME3_GAME_EXE"),
        required_if_eq("enable_unstable_cli", "true"),
        requires("enable_unstable_cli")
    )]
    exe: Option<PathBuf>,

    /// Path to the me3 that should be attached to the game.
    #[arg(
        short('h'),
        long("host-dll"),
        env("ME3_DLL"),
        required_if_eq("enable_unstable_cli", "true"),
        requires("enable_unstable_cli")
    )]
    host_dll: Option<PathBuf>,

    /// A list of paths to ``ModProfile`` configuration files.
    #[arg(
        short('p'),
        long("profile"),
        env("ME3_PROFILE"),
        action = clap::ArgAction::Append,
        requires("enable_unstable_cli")
    )]
    profiles: Vec<PathBuf>,

    #[arg(short('?'), long("help"), action(ArgAction::Help))]
    _help: (),
}

#[tracing::instrument]
fn run() -> LauncherResult<()> {
    info!("Launcher started");

    let args = match LauncherArgs::try_parse() {
        Ok(args) => args,
        Err(error) => {
            error!(%error);
            error.exit();
        }
    };

    if args.profiles.is_empty() {
        info!("No profiles provided");
    } else {
        info!(profiles=?args.profiles, "Loading profiles");
    }

    let mut all_natives = vec![];
    let mut all_packages = vec![];

    for profile_path in args.profiles {
        let base = profile_path
            .parent()
            .and_then(|parent| parent.normalize().ok())
            .ok_or_eyre("failed to normalize base directory for mod profile")?;

        let profile = ModProfile::from_file(&profile_path)?;
        // TODO: check profile.supports

        let mut packages = profile.packages();
        let mut natives = profile.natives();

        packages
            .iter_mut()
            .for_each(|pkg| pkg.source_mut().make_absolute(base.as_path()));
        natives
            .iter_mut()
            .for_each(|pkg| pkg.source_mut().make_absolute(base.as_path()));

        all_packages.extend(packages);
        all_natives.extend(natives);
    }

    let ordered_natives = sort_dependencies(all_natives)?;
    let ordered_packages = sort_dependencies(all_packages)?;

    let (monitor_server, monitor_name) = IpcOneShotServer::new()?;
    let request = AttachRequest {
        monitor_name,
        natives: ordered_natives,
        packages: ordered_packages,
    };

    info!(exe = ?args.exe, dll = ?args.host_dll, "Starting game");

    let shutdown_requested = Arc::new(AtomicBool::new(false));

    let game_exe = args.exe.unwrap();
    let game_path = game_exe.parent();
    let mut game = Game::launch(&game_exe, game_path)?;

    let monitor_thread = std::thread::spawn(move || {
        info!("Starting monitor thread");
        let (receiver, client) = monitor_server.accept().unwrap();
        info!("Host connected to monitor with message {:?}", client);

        loop {
            match receiver.recv() {
                Ok(msg) => match msg {
                    HostMessage::Trace {
                        level,
                        message,
                        target,
                    } => match level.0 {
                        tracing_core::Level::DEBUG => debug!(target = target, message),
                        tracing_core::Level::TRACE => trace!(target = target, message),
                        tracing_core::Level::INFO => info!(target = target, message),
                        tracing_core::Level::WARN => warn!(target = target, message),
                        tracing_core::Level::ERROR => error!(target = target, message),
                    },
                    HostMessage::CrashDumpRequest {
                        exception_pointers,
                        process_id,
                        thread_id,
                        exception_code,
                    } => {
                        info!(exception_code, "Host requested a crashdump");

                        let start = SystemTime::now();
                        let timestamp = start
                            .duration_since(UNIX_EPOCH)
                            .expect("system clock is broken")
                            .as_secs();

                        let file_path = format!("me3_crash_{timestamp}.dmp");
                        let mut file =
                            File::create_new(&file_path).expect("unable to create crash dump file");

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

                        #[cfg(feature = "sentry")]
                        sentry::with_scope(
                            move |scope| {
                                // Remove event.process because this event came from the
                                // main app process
                                scope.remove_extra("event.process");

                                if let Ok(buffer) = std::fs::read(&file_path) {
                                    scope.add_attachment(Attachment {
                                        buffer,
                                        filename: "minidump.dmp".to_string(),
                                        ty: Some(AttachmentType::Minidump),
                                        ..Default::default()
                                    });
                                }
                            },
                            || {
                                sentry::capture_event(Event {
                                    level: Level::Fatal,
                                    ..Default::default()
                                })
                            },
                        );
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

    if let Err(e) = game.attach(&args.host_dll.expect("no host DLL option set"), request) {
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
    #[cfg(feature = "sentry")]
    let _sentry = {
        let sentry_dsn = option_env!("SENTRY_DSN").and_then(|dsn| Dsn::from_str(dsn).ok());

        if sentry_dsn.is_none() {
            warn!("No Sentry DSN provided, but crash reporting was enabled");
        }

        sentry::init(sentry::ClientOptions {
            debug: cfg!(debug_assertions),
            traces_sample_rate: 1.0,
            dsn: sentry_dsn,

            ..Default::default()
        })
    };

    install_tracing();

    run().expect("Failed to successfully run launcher");
}
