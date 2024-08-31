use std::fmt::{Debug, Formatter};

use ipc_channel::ipc::IpcSender;
use me3_mod_protocol::{native::Native, package::Package};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct AttachRequest {
    pub monitor_name: String,

    /// An ordered list of natives to be loaded on attach.
    pub natives: Vec<Native>,

    /// An ordered list of packages to be loaded on attach.
    pub packages: Vec<Package>,

}

#[derive(Deserialize, Serialize)]
pub struct Attachment;

pub type AttachResult = Result<Attachment, AttachError>;

pub type AttachFunction = fn(AttachRequest) -> AttachResult;

#[derive(Debug, Deserialize, Serialize)]
pub struct AttachError(pub String);

impl From<eyre::Report> for AttachError {
    fn from(value: eyre::Report) -> Self {
        AttachError(format!("{:#?}", value ))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum HostMessage {
    Attached,
    Trace(String),
    CrashDumpRequest {
        /// The address of an `EXCEPTION_POINTERS` in the client's memory
        exception_pointers: u64,
        /// The process id of the client process
        process_id: u32,
        /// The id of the thread in the client process in which the crash originated
        thread_id: u32,
        /// The top level exception code, also found in the
        /// `EXCEPTION_POINTERS.ExceptionRecord.ExceptionCode`
        exception_code: i32,
    },
}

pub enum MonitorMessageKind {
    TraceEvent,
}

pub trait MonitorClient: Send + Sync {
    fn send_message(&self, kind: HostMessage);
}

pub trait MonitorServer {
    fn handle_message(&self, kind: HostMessage);
}

impl TryFrom<u32> for MonitorMessageKind {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(MonitorMessageKind::TraceEvent),
            _ => Err(()),
        }
    }
}
