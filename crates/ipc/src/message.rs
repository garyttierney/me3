use rkyv::{Archive, Deserialize, Serialize};

use crate::request::{Request, RequestError, RequestId, Response};

/// Messages that can be received by
/// [`BridgeToChild::recv_loop`](crate::bridge::BridgeToChild::recv_loop).
#[derive(Clone, Archive, Serialize, Deserialize)]
pub enum MsgToParent {
    /// A single log message from the child process.
    Log(Box<str>),

    /// RPC response coming from the child process.
    ///
    /// The message thread must use [`Response::forward`](crate::request::Response::forward) to
    /// unblock the thread waiting on the response.
    Response((RequestId, Result<Response, RequestError>)),

    /// A hint to flush the log buffer, if there is one.
    Flush,
}

/// Messages that can be received by
/// [`BridgeToParent::recv_loop`](crate::bridge::BridgeToParent::recv_loop).
#[derive(Clone, Archive, Serialize, Deserialize)]
pub enum MsgToChild {
    /// RPC request coming from the parent process.
    ///
    /// See [`BridgeToParent::fulfill`](crate::bridge::BridgeToParent::fulfill).
    Request((RequestId, Request)),
}
