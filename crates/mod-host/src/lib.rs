#![feature(fn_traits)]
#![feature(fn_ptr_trait)]
#![feature(tuple_trait)]
#![feature(unboxed_closures)]

use std::{
    cell::UnsafeCell,
    fs::OpenOptions,
    io::PipeWriter,
    os::windows::prelude::{FromRawHandle, RawHandle},
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use crash_handler::CrashEventResult;
use me3_launcher_attach_protocol::{AttachRequest, AttachResult, Attachment, HostMessage};
use me3_mod_host_assets::mapping::ArchiveOverrideMapping;
use tracing::info;

use crate::host::{hook::thunk::ThunkPool, ModHost};

mod asset_archive;
mod detour;
mod host;

static INSTANCE: OnceLock<usize> = OnceLock::new();
/// https://learn.microsoft.com/en-us/windows/win32/dlls/dllmain#parameters
const DLL_PROCESS_ATTACH: u32 = 1;

dll_syringe::payload_procedure! {
    fn me_attach(request: AttachRequest) -> AttachResult {
        on_attach(request)
    }
}

fn on_attach(request: AttachRequest) -> AttachResult {
    let AttachRequest {
        monitor_pipe,
        config,
    } = request;

    let mut socket = unsafe { PipeWriter::from_raw_handle(monitor_pipe.0 as *mut _) };

    HostMessage::Attached.write(&mut socket);

    // crash_handler::CrashHandler::attach(unsafe {
    //     crash_handler::make_crash_event(move |crash_context: &crash_handler::CrashContext| {
    //         info!("Handling crash event");

    //         let request = HostMessage::CrashDumpRequest {
    //             exception_pointers: crash_context.exception_pointers as u64,
    //             process_id: crash_context.process_id,
    //             thread_id: crash_context.thread_id,
    //             exception_code: crash_context.exception_code,
    //         };

    //         let _ = request.write(socket.as_mut_unchecked());

    //         std::thread::sleep(Duration::from_secs(5));
    //         CrashEventResult::Handled(false)
    //     })
    // })?;

    let log_file_path = std::env::var("ME3_LOG_FILE").expect("log file location not set");
    let log_file = OpenOptions::new()
        .append(true)
        .open(log_file_path)
        .expect("couldn't open log file");

    let telemetry = me3_telemetry::install(std::env::var("ME3_TELEMETRY").is_ok(), move || {
        log_file.try_clone().unwrap()
    });

    info!("Host monitoring configured");

    let mut host = ModHost::new(telemetry, ThunkPool::new()?);

    for native in config.natives {
        host.load_native(&native.path, native.initializer)?;
    }

    let mut override_mapping = ArchiveOverrideMapping::default();
    override_mapping.scan_directories(config.packages.iter())?;
    asset_archive::attach(&mut host, Arc::new(override_mapping))?;

    host.attach();

    info!("Host successfully attached");

    Ok(Attachment)
}

#[no_mangle]
pub extern "stdcall" fn DllMain(instance: usize, reason: u32, _: *mut usize) -> i32 {
    if reason == DLL_PROCESS_ATTACH {
        let _ = INSTANCE.set(instance);
    }

    1
}
