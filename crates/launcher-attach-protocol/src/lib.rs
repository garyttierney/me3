use me3_mod_protocol::{native::Native, package::Package};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct AttachRequest {
    /// An ordered list of natives to be loaded on attach.
    pub natives: Vec<Native>,

    /// An ordered list of packages to be loaded on attach.
    pub packages: Vec<Package>,
}

#[derive(Deserialize, Serialize)]
pub struct Attachment;

pub type AttachResult = Result<Attachment, AttachError>;

pub type AttachFunction = fn(AttachRequest) -> AttachResult;

#[derive(Deserialize, Serialize)]
pub struct AttachError(pub String);

impl From<eyre::Report> for AttachError {
    fn from(value: eyre::Report) -> Self {
        AttachError(value.to_string())
    }
}
