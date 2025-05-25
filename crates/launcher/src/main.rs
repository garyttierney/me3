use std::{
    env,
    fs::File,
    io,
    path::PathBuf,
    process::exit,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering::SeqCst},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use crash_context::CrashContext;
use eyre::{Context, OptionExt};
use ipc_channel::ipc::{IpcError, IpcOneShotServer};
use me3_launcher_attach_protocol::{AttachRequest, HostMessage};
use me3_mod_protocol::{dependency::sort_dependencies, package::WithPackageSource, ModProfile};
use minidump_writer::minidump_writer::MinidumpWriter;
use sentry::types::Dsn;
#[cfg(feature = "sentry")]
use sentry::{
    protocol::{Attachment, AttachmentType, Event},
    Level,
};
use tracing::{error, info, warn};

use crate::game::Game;

mod game;

pub type LauncherResult<T> = stable_eyre::Result<T>;

/// Launch a Steam game with the me3 mod loader attached.
struct LauncherArgs {
    /// Path to the game EXE that should be launched.
    exe: PathBuf,

    /// Path to the me3 that should be attached to the game.
    dll: PathBuf,

    /// A list of paths to ModProfile configuration files.
    profiles: Vec<PathBuf>,
}

impl LauncherArgs {
    pub fn from_env() -> Result<Self, eyre::Error> {
        let exe = PathBuf::from(env::var("ME3_GAME_EXE").wrap_err("game exe wasn't set")?);
        let dll = PathBuf::from(env::var("ME3_DLL").wrap_err("me3 host dll wasn't set")?);
        let profiles = env::args().skip(1).map(PathBuf::from).collect();

        Ok(LauncherArgs { exe, dll, profiles })
    }
}

fn run() -> LauncherResult<()> {
    info!("Launcher started");

    let args = match LauncherArgs::from_env() {
        Ok(args) => args,
        Err(e) => {
            error!(%e);
            exit(1)
        }
    };

    #[cfg(feature = "sentry")]
    let _sentry = {
        let sentry_dsn = option_env!("SENTRY_DSN").and_then(|dsn| Dsn::from_str(dsn).ok());

        if sentry_dsn.is_none() {
            warn!("No Sentry DSN provider, but crash reporting was enabled");
        }

        sentry::init(sentry::ClientOptions {
            release: sentry::release_name!(),
            dsn: sentry_dsn,
            ..Default::default()
        })
    };

    if args.profiles.is_empty() {
        info!("No profiles provided");
    } else {
        info!("Loading profiles from {:?}", args.profiles);
    }

    let all_natives = vec![];
    let all_packages = vec![];

    for profile_path in args.profiles {
        let base = profile_path
            .parent()
            .and_then(|parent| parent.canonicalize().ok())
            .ok_or_eyre("failed to acquire base directory for mod profile")?;

        let profile = ModProfile::from_file(&profile_path)?;
        // TODO: check profile.supports

        let mut packages = profile.packages();
        let mut natives = profile.natives();

        packages
            .iter_mut()
            .for_each(|pkg| pkg.source_mut().make_absolute(&base));
        natives
            .iter_mut()
            .for_each(|pkg| pkg.source_mut().make_absolute(&base));
    }

    let ordered_natives = sort_dependencies(all_natives)?;
    let ordered_packages = sort_dependencies(all_packages)?;

    let (monitor_server, monitor_name) = IpcOneShotServer::new()?;
    let request = AttachRequest {
        monitor_name,
        natives: ordered_natives,
        packages: ordered_packages,
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
