#![windows_subsystem = "windows"]

use std::{
    env,
    error::Error,
    fs::{File, OpenOptions},
    path::PathBuf,
    sync::{
        atomic::{
            AtomicBool,
            Ordering::{self, SeqCst},
        },
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use crash_context::CrashContext;
use eyre::Context;
use ipc_channel::ipc::{IpcError, IpcOneShotServer};
use me3_launcher_attach_protocol::{AttachRequest, HostMessage};
use minidump_writer::minidump_writer::MinidumpWriter;
#[cfg(feature = "sentry")]
use sentry::{
    protocol::{Attachment, AttachmentType, Event},
    Level,
};
use tracing::{error, info, info_span};
use tracing_subscriber::fmt::writer::BoxMakeWriter;

use crate::game::Game;

mod game;

pub type LauncherResult<T> = eyre::Result<T>;

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

    let (monitor_server, monitor_name) = IpcOneShotServer::new()?;
    let request = AttachRequest {
        monitor_name,
        config: attach_config,
    };

    info!("Starting game at {:?} with DLL {:?}", args.exe, args.dll);

    let shutdown_requested = Arc::new(AtomicBool::new(false));

    let game_path = args.exe.parent();
    let mut game = Game::launch(&args.exe, game_path)?;

    let monitor_thread_shutdown = shutdown_requested.clone();
    let monitor_thread = std::thread::spawn(move || {
        info!("Starting monitor thread");
        let (receiver, client) = monitor_server.accept().unwrap();
        info!("Host connected to monitor with message {:?}", client);

        while monitor_thread_shutdown.load(Ordering::SeqCst) {
            match receiver.recv() {
                Ok(msg) => match msg {
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
                        .expect("failed to write crash dump to file");

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
                    error!(error = &e as &dyn Error, "Error from monitor channel");
                    break;
                }
            }
        }
    });

    if let Err(e) = game.attach(&args.dll, request) {
        error!(error = &*e, "Failed to attach to game");
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
        .create(true)
        .open(log_file_path)
        .expect("couldn't open log file");

    let monitor_log_file_path =
        std::env::var("ME3_MONITOR_LOG_FILE").expect("log file location not set");

    let monitor_log_file = OpenOptions::new()
        .append(true)
        .open(monitor_log_file_path)
        .expect("couldn't open log file");

    let _telemetry = me3_telemetry::install(
        std::env::var("ME3_TELEMETRY").is_ok(),
        Some(BoxMakeWriter::new(log_file)),
        Some(BoxMakeWriter::new(monitor_log_file)),
    );

    if let Err(e) = run() {
        error!(?e, "Failed to run launcher: {e}");
    }
}
