use std::{
    fmt::Debug,
    io::{Read, Write},
    path::PathBuf,
};

use bincode::{error::DecodeError, Decode, Encode};
use me3_mod_protocol::{native::Native, package::Package, Game};
use rkyv::{
    option::ArchivedOption,
    rancor::{Fallible, Source},
    string::ArchivedString,
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, SerializeUnsized,
};
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,
)]
pub struct AttachRequest {
    pub config: AttachConfig,
}

#[derive(
    Clone, Debug, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct AttachConfig {
    /// The attached to game.
    pub game: Game,

    /// An ordered list of natives to be loaded on attach.
    pub natives: Vec<Native>,

    /// An ordered list of natives to be loaded early on attach.
    pub early_natives: Vec<Native>,

    /// An ordered list of packages to be loaded on attach.
    pub packages: Vec<Package>,

    /// Name of an alternative savefile to use (in the default savefile directory).
    pub savefile: Option<String>,

    #[rkyv(with = AsOptionString)]
    /// Path to the cache directory.
    pub cache_path: Option<PathBuf>,

    /// Suspend the game until a debugger is attached?
    pub suspend: bool,

    /// Cache decrypted BHD files to improve game startup speed?
    pub boot_boost: bool,

    /// Skip the intro logos shown on every game launch?
    pub skip_logos: bool,

    /// Allow multiplayer server access?
    pub start_online: bool,

    /// Try to neutralize Arxan code protection to improve mod stability?
    pub disable_arxan: bool,

    /// Patch memory limits for supported games.
    pub mem_patch: bool,

    /// Should we avoid checking if Steam is running as part of pre-launch checks?
    pub skip_steam_init: bool,
}

struct AsOptionString;

impl ArchiveWith<Option<PathBuf>> for AsOptionString {
    type Archived = ArchivedOption<ArchivedString>;
    type Resolver = <Option<String> as Archive>::Resolver;

    fn resolve_with(
        field: &Option<PathBuf>,
        resolver: Self::Resolver,
        out: rkyv::Place<Self::Archived>,
    ) {
        Option::<String>::resolve(
            &field.clone().map(|path| path.to_string_lossy().to_string()),
            resolver,
            out,
        );
    }
}

impl<S: Fallible + ?Sized> SerializeWith<Option<PathBuf>, S> for AsOptionString
where
    S::Error: Source,
    str: SerializeUnsized<S>,
{
    fn serialize_with(
        field: &Option<PathBuf>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        rkyv::Serialize::serialize(
            &field.clone().map(|path| path.to_string_lossy().to_string()),
            serializer,
        )
    }
}

impl<D> DeserializeWith<ArchivedOption<ArchivedString>, Option<PathBuf>, D> for AsOptionString
where
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedOption<ArchivedString>,
        _: &mut D,
    ) -> Result<Option<PathBuf>, D::Error> {
        match field {
            ArchivedOption::Some(field) => Ok(Some(PathBuf::from(field.as_str()))),
            ArchivedOption::None => Ok(None),
        }
    }
}

#[derive(
    Clone, Debug, Deserialize, Serialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct Attachment;

pub type AttachResult = Result<Attachment, AttachError>;

pub type AttachFunction = fn(AttachRequest) -> AttachResult;

#[derive(
    Clone, Debug, Deserialize, Serialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct AttachError(pub String);

impl<E: Into<eyre::Report>> From<E> for AttachError {
    fn from(value: E) -> Self {
        let err = value.into();
        AttachError(format!("{err:#?}"))
    }
}

#[derive(Debug, Decode, Encode)]
pub enum HostMessage {
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
