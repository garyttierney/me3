use rkyv::{Archive, Deserialize, Serialize};

use crate::request::{Request, RequestError, RequestId, Response};

#[derive(Clone, Archive, Serialize, Deserialize)]
pub enum MsgToParent {
    Log(Box<str>),
    Response((RequestId, Result<Response, RequestError>)),
    Flush,
}

#[derive(Clone, Archive, Serialize, Deserialize)]
pub enum MsgToChild {
    Request((RequestId, Request)),
}
