use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct AttachRequest {
    pub name: String,
}

#[derive(Deserialize, Serialize)]
pub struct AttachResponse {}

pub type AttachFunction = fn(AttachRequest) -> AttachResponse;
