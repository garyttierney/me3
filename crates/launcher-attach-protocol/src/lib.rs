use std::{
    fmt::Debug,
    io::{Read, Write},
};

use bincode::{error::DecodeError, Decode, Encode};
use me3_mod_protocol::{native::Native, package::Package, Game};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct AttachRequest {
    pub monitor_handle: usize,

    pub config: AttachConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AttachConfig {
    /// The attached to game.
    pub game: Game,

    /// An ordered list of natives to be loaded on attach.
    pub natives: Vec<Native>,

    /// An ordered list of packages to be loaded on attach.
    pub packages: Vec<Package>,

    /// Suspend the game until a debugger is attached?
    pub suspend: bool,

    /// Should we avoid checking if Steam is running as part of pre-launch checks?
    pub skip_steam_init: bool,
}

#[derive(Debug, Deserialize, Serialize)]
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
    pub fn read_from(reader: &mut impl Read) -> std::io::Result<Self> {
        bincode::decode_from_std_read(reader, bincode::config::standard()).map_err(
            |err| match err {
                DecodeError::Io { inner, .. } => inner,
                err => std::io::Error::other(err),
            },
        )
    }

    pub fn write_to(self, writer: &mut impl Write) -> std::io::Result<usize> {
        bincode::encode_into_std_write(&self, writer, bincode::config::standard())
            .map_err(std::io::Error::other)
    }
}
