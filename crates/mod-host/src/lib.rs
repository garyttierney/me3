#![feature(fn_traits)]
#![feature(fn_ptr_trait)]
#![feature(tuple_trait)]
#![feature(unboxed_closures)]
#![feature(naked_functions)]

use std::{
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use crash_handler::CrashEventResult;
use futures_util::future::err;
use ipc_channel::ipc::IpcSender;
use log::error;
use me3_launcher_attach_protocol::{
    AttachError, AttachRequest, AttachResult, Attachment, HostMessage,
};
use minidumper::Client;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, FmtSubscriber};

use crate::{
    diagnostics::HostTracingLayer,
    host::{hook::thunk::ThunkPool, ModHost},
};

mod detour;
mod diagnostics;
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
    let AttachRequest { monitor_name, .. } = request;

    let socket = IpcSender::connect(monitor_name).unwrap();
    let mut socket = Arc::new(Mutex::new(socket));
    let mut crash_handler_socket = socket.clone();

    let crash_handler = crash_handler::CrashHandler::attach(unsafe {
        crash_handler::make_crash_event(move |crash_context: &crash_handler::CrashContext| {
            info!("Handling crash event");
            let _ = crash_handler_socket
                .lock()
                .unwrap()
                .send(HostMessage::CrashDumpRequest {
                    exception_pointers: crash_context.exception_pointers as u64,
                    process_id: crash_context.process_id,
                    thread_id: crash_context.thread_id,
                    exception_code: crash_context.exception_code,
                });

            // TODO: we need a safe way keep the process alive until the minidump is captured.
            std::thread::sleep(Duration::from_secs(5));

            CrashEventResult::Handled(false)
        })
    })
    .expect("failed to attach crash handler");


    socket.lock().unwrap().send(HostMessage::Attached).unwrap();

    tracing_subscriber::registry()
        .with(HostTracingLayer { socket })
        .init();

    info!("Host monitoring configured");

    let host = ModHost::new(crash_handler, ThunkPool::new()?);
    host.panic();
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
