use std::any::type_name;

use me3_launcher_attach_protocol::{AttachRequest, AttachResult};
use rkyv::{Archive, Deserialize, Serialize};

use crate::request::{Request, Response};

/// Trait for converting distinct types into [`Request`]s.
///
/// Also associates it with a corresponding [`ConvertResponse`].
pub trait ConvertRequest: Sized {
    type Res: ConvertResponse;

    fn into_req(self) -> Request;
    fn try_from_req(req: Request) -> Result<Self, TryFromRequestError>;
}

/// Trait for converting distinct types into [`Response`]s.
///
/// Also associates it with a corresponding [`ConvertRequest`].
pub trait ConvertResponse: Sized {
    type Req: ConvertRequest;

    fn into_res(self) -> Response;
    fn try_from_res(res: Response) -> Result<Self, TryFromResponseError>;
}

#[derive(Clone, Debug, thiserror::Error, Archive, Serialize, Deserialize)]
#[error("expected {expected}")]
pub struct TryFromError {
    pub expected: Box<str>,
}

#[derive(Clone, Debug, thiserror::Error, Archive, Serialize, Deserialize)]
#[error(transparent)]
pub struct TryFromRequestError(#[from] pub TryFromError);

#[derive(Clone, Debug, thiserror::Error, Archive, Serialize, Deserialize)]
#[error(transparent)]
pub struct TryFromResponseError(#[from] pub TryFromError);

impl TryFromError {
    fn err<T, E: From<Self>>() -> E {
        Self {
            expected: type_name::<T>().into(),
        }
        .into()
    }
}

impl ConvertRequest for AttachRequest {
    type Res = AttachResult;

    fn into_req(self) -> Request {
        Request::Attach(self)
    }

    fn try_from_req(req: Request) -> Result<Self, TryFromRequestError> {
        match req {
            Request::Attach(req) => Ok(req),
            _ => Err(TryFromError::err::<Self, _>()),
        }
    }
}

impl ConvertResponse for AttachResult {
    type Req = AttachRequest;

    fn into_res(self) -> Response {
        Response::Attach(self)
    }

    fn try_from_res(res: Response) -> Result<Self, TryFromResponseError> {
        match res {
            Response::Attach(res) => Ok(res),
            _ => Err(TryFromError::err::<Self, _>()),
        }
    }
}
