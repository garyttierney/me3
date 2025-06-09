#![windows_subsystem = "windows"]

use std::{
    error::Error,
    fs::File,
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
use me3_env::{LauncherVars, TelemetryVars};
use me3_launcher_attach_protocol::{AttachRequest, HostMessage};
use me3_telemetry::TelemetryConfig;
use minidump_writer::minidump_writer::MinidumpWriter;
use tracing::{error, info};

use crate::game::Game;

mod game;

pub type LauncherResult<T> = eyre::Result<T>;

fn run() -> LauncherResult<()> {
    info!("Launcher started");

    let args: LauncherVars = me3_env::deserialize_from_env()?;

    info!(?args, "Parsed launcher args");

    let attach_config_text = std::fs::read_to_string(args.host_config_path)?;
    let attach_config = toml::from_str(&attach_config_text)?;

    let (monitor_server, monitor_name) = IpcOneShotServer::new()?;
    let request = AttachRequest {
        monitor_name,
        config: attach_config,
    };

    info!(
        "Starting game at {:?} with DLL {:?}",
        args.exe, args.host_dll
    );

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

                        // #[cfg(feature = "sentry")]
                        // sentry::with_scope(
                        //     move |scope| {
                        //         // Remove event.process because this event came from the
                        //         // main app process
                        //         scope.remove_extra("event.process");

                        //         if let Ok(buffer) = std::fs::read(&file_path) {
                        //             scope.add_attachment(Attachment {
                        //                 buffer,
                        //                 filename: "minidump.dmp".to_string(),
                        //                 ty: Some(AttachmentType::Minidump),
                        //                 ..Default::default()
                        //             });
                        //         }
                        //     },
                        //     || {
                        //         sentry::capture_event(Event {
                        //             level: Level::Fatal,
                        //             ..Default::default()
                        //         })
                        //     },
                        // );
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

    let result = game.attach(&args.host_dll, request);

    game.join();
    shutdown_requested.store(true, SeqCst);
    let _ = monitor_thread.join();

    result.map(|_| ())
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
