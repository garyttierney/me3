#![allow(unreachable_patterns)]

use std::{
    collections::HashMap,
    hash::{BuildHasher, Hash, Hasher, RandomState},
    hint,
    marker::PhantomPinned,
    panic::{self, UnwindSafe},
    pin::{pin, Pin},
    sync::{
        atomic::{AtomicBool, Ordering},
        Condvar, Mutex,
    },
};

use me3_launcher_attach_protocol::{AttachRequest, AttachResult};
use rkyv::{Archive, Deserialize, Serialize};

use crate::{
    bridge::SendError,
    identity_hasher::IdentityBuildHasher,
    request::convert::{
        ConvertRequest, ConvertResponse, TryFromRequestError, TryFromResponseError,
    },
};

pub mod convert;

/// Opaque unique request ID.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub struct RequestId(u64);

/// Kinds of requests seen by
/// [`BridgeToParent::recv_loop`](crate::bridge::BridgeToParent::recv_loop).
#[derive(Clone, Archive, Serialize, Deserialize)]
pub enum Request {
    Attach(AttachRequest),
}

/// Kinds of responses seen by
/// [`BridgeToChild::recv_loop`](crate::bridge::BridgeToChild::recv_loop).
#[derive(Clone, Archive, Serialize, Deserialize)]
pub enum Response {
    Attach(AttachResult),
}

#[derive(Clone, Debug, thiserror::Error, Archive, Serialize, Deserialize)]
pub enum RequestError {
    #[error("failed to send: {0}")]
    Send(Box<str>),

    #[error("bad request: {0}")]
    BadRequest(#[from] TryFromRequestError),

    #[error("bad response: {0}")]
    BadResponse(#[from] TryFromResponseError),

    #[error("tried to send a request from the receive loop")]
    RequestFromRecv,

    #[error("request panicked: {0}")]
    Panic(Box<str>),
}

impl Request {
    pub(crate) fn fulfill<Req, F>(self, f: F) -> Result<Req::Res, RequestError>
    where
        Req: ConvertRequest + UnwindSafe,
        F: FnOnce(Req) -> Req::Res + UnwindSafe,
    {
        let req = Req::try_from_req(self)?;

        // Propagate panics as errors to the parent process.
        match panic::catch_unwind(move || f(req)) {
            Ok(res) => Ok(res),
            Err(payload) => {
                let panic_msg = payload
                    .downcast::<&'static str>()
                    .map_or("no panic message", |boxed| *boxed);
                Err(RequestError::Panic(panic_msg.into()))
            }
        }
    }

    pub(crate) fn await_response<Res, F>(self, id: RequestId, f: F) -> Result<Res, RequestError>
    where
        F: FnOnce((RequestId, Self)) -> Result<(), SendError>,
        Res: ConvertResponse,
    {
        // Pin on the stack - we'll block until the response is ready or an error occurs.
        let res: Pin<&AwaitedResponse> = pin!(AwaitedResponse::new(id));

        // Register (the id should be unique).
        res.register();
        f((id, self)).map_err(|e| RequestError::Send(e.to_string().into_boxed_str()))?;

        // Block until the response is forwarded.
        res.await_response()
    }

    pub(crate) fn generate_id() -> RequestId {
        let mut hasher = RandomState::new().build_hasher();
        std::thread::current().id().hash(&mut hasher);
        std::time::Instant::now().hash(&mut hasher);
        RequestId(hasher.finish())
    }
}

impl Response {
    /// Forward the response to the waiting thread to unblock it.
    pub fn forward(res: (RequestId, Result<Self, RequestError>)) {
        AwaitedResponse::forward_response(res);
    }
}

struct AwaitedResponse {
    id: RequestId,
    payload: (Mutex<Option<Result<Response, RequestError>>>, Condvar),
    is_registered: AtomicBool,
    is_fulfilled: AtomicBool,
    _marker: PhantomPinned,
}

struct AwaitedResponsePtr(*const AwaitedResponse);

// Identity hasher because `RequestId` are already hashes (and are unique).
static AWAITED_RESPONSES: Mutex<HashMap<RequestId, AwaitedResponsePtr, IdentityBuildHasher>> =
    Mutex::new(HashMap::with_hasher(IdentityBuildHasher));

impl AwaitedResponse {
    const SPIN_COUNT: u32 = 1000;

    fn new(id: RequestId) -> Self {
        Self {
            id,
            payload: (Mutex::new(None), Condvar::new()),
            is_registered: AtomicBool::new(false),
            is_fulfilled: AtomicBool::new(false),
            _marker: PhantomPinned,
        }
    }

    fn register(self: Pin<&Self>) {
        let with_this_id = AWAITED_RESPONSES
            .lock()
            .unwrap()
            .insert(self.id, AwaitedResponsePtr(self.get_ref()));

        assert!(
            with_this_id.is_none(),
            "request with this id was already registered"
        );

        self.is_registered.store(true, Ordering::Relaxed);
    }

    fn await_response<Res>(self: Pin<&Self>) -> Result<Res, RequestError>
    where
        Res: ConvertResponse,
    {
        // Start by spinning.
        for _ in 0..Self::SPIN_COUNT {
            if self.is_fulfilled.load(Ordering::Relaxed) {
                let mut lock = self.payload.0.lock().unwrap();
                let res = Res::try_from_res(lock.take().unwrap()?)?;
                return Ok(res);
            }
            hint::spin_loop();
        }

        // Wait on a condition variable.
        let (lock, cvar) = &self.payload;
        let mut res = lock.lock().unwrap();
        loop {
            if let Some(res) = res.take() {
                let res = Res::try_from_res(res?)?;
                return Ok(res);
            }
            res = cvar.wait(res).unwrap();
        }
    }

    fn forward_response((id, res): (RequestId, Result<Response, RequestError>)) {
        let mut awaited_responses = AWAITED_RESPONSES.lock().unwrap();

        // Remove the request directly, it saves removing it in the `AwaitedResponse` drop impl.
        let Some(awaited) = awaited_responses.remove(&id).map(|ptr| unsafe { &*ptr.0 }) else {
            // Unexpected response?
            return;
        };

        // Unregister, fulfill and notify.
        awaited.is_registered.store(false, Ordering::Relaxed);

        let (lock, cvar) = &awaited.payload;
        *lock.lock().unwrap() = Some(res);

        awaited.is_fulfilled.store(true, Ordering::Relaxed);
        cvar.notify_all();
    }
}

impl Drop for AwaitedResponse {
    fn drop(&mut self) {
        // If we are still registered, remove the request.
        if *self.is_registered.get_mut() {
            // This only happens if the request isn't fulfilled successfully.
            let _ = AWAITED_RESPONSES.lock().unwrap().remove(&self.id);
        }
    }
}

unsafe impl Send for AwaitedResponsePtr {}

unsafe impl Sync for AwaitedResponsePtr {}
