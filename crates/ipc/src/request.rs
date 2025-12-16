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
    request::convert::{
        ConvertRequest, ConvertResponse, TryFromRequestError, TryFromResponseError,
    },
};

pub mod convert;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub struct RequestId(u64);

#[derive(Clone, Archive, Serialize, Deserialize)]
pub enum Request {
    Attach(AttachRequest),
}

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

    #[error("request panicked: {0}")]
    Panic(Box<str>),
}

impl Request {
    #[inline]
    pub(crate) fn fulfill<Req, F>(self, f: F) -> Result<Req::Res, RequestError>
    where
        Req: ConvertRequest + UnwindSafe,
        F: FnOnce(Req) -> Req::Res + UnwindSafe,
    {
        let req = Req::try_from_req(self)?;
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

    #[inline]
    pub(crate) fn await_response<Res, F>(self, id: RequestId, f: F) -> Result<Res, RequestError>
    where
        F: FnOnce((RequestId, Self)) -> Result<(), SendError>,
        Res: ConvertResponse,
    {
        let res: Pin<&AwaitedResponse> = pin!(AwaitedResponse::new(id));
        res.register();
        f((id, self)).map_err(|e| RequestError::Send(e.to_string().into_boxed_str()))?;
        res.await_response()
    }

    #[inline]
    pub(crate) fn generate_id() -> RequestId {
        let mut hasher = RandomState::new().build_hasher();
        std::thread::current().id().hash(&mut hasher);
        std::time::Instant::now().hash(&mut hasher);
        RequestId(hasher.finish())
    }
}

impl Response {
    #[inline]
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

static AWAITED_RESPONSES: Mutex<HashMap<RequestId, AwaitedResponsePtr, RequestBuildHasher>> =
    Mutex::new(HashMap::with_hasher(RequestBuildHasher));

impl AwaitedResponse {
    const SPIN_COUNT: u32 = 1000;

    #[inline]
    fn new(id: RequestId) -> Self {
        Self {
            id,
            payload: (Mutex::new(None), Condvar::new()),
            is_registered: AtomicBool::new(false),
            is_fulfilled: AtomicBool::new(false),
            _marker: PhantomPinned,
        }
    }

    #[inline]
    fn register(self: Pin<&Self>) {
        let _ = AWAITED_RESPONSES
            .lock()
            .unwrap()
            .insert(self.id, AwaitedResponsePtr(self.get_ref()));

        self.is_registered.store(true, Ordering::Relaxed);
    }

    #[inline]
    fn await_response<Res>(self: Pin<&Self>) -> Result<Res, RequestError>
    where
        Res: ConvertResponse,
    {
        for _ in 0..Self::SPIN_COUNT {
            if self.is_fulfilled.load(Ordering::Relaxed) {
                let mut lock = self.payload.0.lock().unwrap();
                let res = Res::try_from_res(lock.take().unwrap()?)?;
                return Ok(res);
            }
            hint::spin_loop();
        }

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

    #[inline]
    fn forward_response((id, res): (RequestId, Result<Response, RequestError>)) {
        let mut awaited_responses = AWAITED_RESPONSES.lock().unwrap();
        let Some(awaited) = awaited_responses.remove(&id).map(|ptr| unsafe { &*ptr.0 }) else {
            return;
        };

        awaited.is_registered.store(false, Ordering::Relaxed);

        let (lock, cvar) = &awaited.payload;
        *lock.lock().unwrap() = Some(res);

        awaited.is_fulfilled.store(true, Ordering::Relaxed);
        cvar.notify_all();
    }
}

impl Drop for AwaitedResponse {
    #[inline]
    fn drop(&mut self) {
        if *self.is_registered.get_mut() {
            let _ = AWAITED_RESPONSES.lock().unwrap().remove(&self.id);
        }
    }
}

struct RequestBuildHasher;

impl BuildHasher for RequestBuildHasher {
    type Hasher = RequestHasher;

    #[inline]
    fn build_hasher(&self) -> Self::Hasher {
        RequestHasher(0)
    }
}

struct RequestHasher(u64);

impl Hasher for RequestHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let a = std::array::from_fn(|i| bytes.get(i).cloned().unwrap_or(0));
        self.0 = u64::from_le_bytes(a);
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

unsafe impl Send for AwaitedResponsePtr {}

unsafe impl Sync for AwaitedResponsePtr {}
