use me3_mod_protocol::ModProfile;
use serde_derive::{Deserialize, Serialize};

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct AttachRequest {
    pub profiles: Vec<ModProfile>,
}

#[derive(Deserialize, Serialize)]
pub struct AttachResponse {}

pub type AttachFunction = fn(AttachRequest) -> AttachResponse;
