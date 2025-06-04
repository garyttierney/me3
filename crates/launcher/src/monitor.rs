use std::{
    fs::File,
    io::PipeReader,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::{SystemTime, UNIX_EPOCH},
};

use crash_context::CrashContext;
use me3_launcher_attach_protocol::HostMessage;
use minidump_writer::minidump_writer::MinidumpWriter;
#[cfg(feature = "sentry")]
use sentry::{
    protocol::{Attachment, AttachmentType, Event},
    Level,
};
use tracing::info;

pub fn run_monitor(
    shutdown: Arc<AtomicBool>,
    mut pipe: PipeReader,
) -> JoinHandle<eyre::Result<()>> {
    std::thread::spawn(move || {
        while !shutdown.load(Ordering::SeqCst) {
            let message = HostMessage::read(&mut pipe)?;

            match message {
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
            };
        }
        Ok(())
    })
}
