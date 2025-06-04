use std::{
    fmt::Debug,
    io::{PipeReader, Read, Write},
};

use bincode::{Decode, Encode};
use me3_mod_protocol::{native::Native, package::Package};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct MonitorPipeHandle(pub usize);

#[derive(Debug, Deserialize, Serialize)]
pub struct AttachRequest {
    pub monitor_pipe: MonitorPipeHandle,

    pub config: AttachConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AttachConfig {
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

impl<E: Into<eyre::Report>> From<E> for AttachError {
    fn from(value: E) -> Self {
        let err = value.into();
        AttachError(format!("{err:#?}"))
    }
}

#[derive(Debug, Decode, Encode)]
pub enum HostMessage {
    Attached,
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

impl HostMessage {
    pub fn read(reader: &mut impl Read) -> std::io::Result<Self> {
        bincode::decode_from_std_read(reader, bincode::config::standard())
            .map_err(std::io::Error::other)
    }

    pub fn write(self, writer: &mut impl Write) -> std::io::Result<usize> {
        bincode::encode_into_std_write(&self, writer, bincode::config::standard())
            .map_err(std::io::Error::other)
    }
}
