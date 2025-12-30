use std::{
    ffi::c_void, io, mem, panic::UnwindSafe, process::Command, ptr::NonNull, sync::LazyLock,
};

use me3_env::{deserialize_from_env, serialize_into_command, EnvVars};
use rkyv::rancor;
use windows::{
    core::{Error as WinError, BOOL, PCWSTR},
    Win32::{
        Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE},
        Security::SECURITY_ATTRIBUTES,
        System::Memory::{
            CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, FILE_MAP_ALL_ACCESS,
            MEMORY_MAPPED_VIEW_ADDRESS, PAGE_READWRITE,
        },
    },
};

pub(crate) use crate::bridge::channel::SendError;
pub use crate::bridge::channel::{RecvError, RecvSpanGuard, SpanError};
use crate::{
    bridge::shared::SharedBridge,
    message::{MsgToChild, MsgToParent},
    request::{
        convert::{ConvertRequest, ConvertResponse},
        Request, RequestError, RequestId,
    },
};

mod buffer;
mod channel;
mod rel;
mod shared;
mod signal;

/// An IPC bridge to the parent process.
///
/// Can be cheaply obtained any number of times with [`to_parent`], but must be preceded by
/// a single call to [`to_child`] in the parent process that creates it.
///
/// This end is [`Clone`] and dropping it does not close the connection.
#[derive(Clone)]
pub struct BridgeToParent {
    shared: &'static SharedBridge,
}

/// An IPC bridge to the child process.
///
/// Creating it with [`to_child`] allocates and sets up the connection.
///
/// Dropping this end closes the connection.
pub struct BridgeToChild {
    shared: &'static SharedBridge,
    _file_view: FileView,
    _file_mapping: FileMapping,
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum BridgeError {
    #[error(transparent)]
    Os(#[from] WinError),

    #[error("JSON deserialization error: {0}")]
    Json(Box<str>),

    #[error("invalid bridge size (`size_mb` is {0})")]
    Size(u32),
}

#[derive(Clone)]
pub struct LogWriter<F: Fn(Box<str>) -> MsgToParent + Clone> {
    bridge: BridgeToParent,
    to_msg: F,
}

/// Opens the child end of the IPC bridge to the parent process.
///
/// Can be cheaply called any number of times, but must be preceded by
/// a single call to [`to_child`] in the parent process that creates it.
pub fn to_parent() -> Result<BridgeToParent, BridgeError> {
    static BRIDGE: LazyLock<Result<BridgeToParent, BridgeError>> = LazyLock::new(|| {
        let file_mapping = deserialize_from_env::<FileMapping>()
            .map_err(|e| BridgeError::Json(e.to_string().into_boxed_str()))?;
        let file_view = file_mapping.to_view()?;

        // SAFETY: the bridge was initialized by the parent process.
        let shared = unsafe { file_view.as_slice().cast::<SharedBridge>().as_ref() };

        // Forget both handles, they will be closed when the process finishes.
        mem::forget(file_view);
        mem::forget(file_mapping);

        Ok(BridgeToParent { shared })
    });
    BRIDGE.clone()
}

/// Creates the parent end of the IPC bridge to the child process.
///
/// This function allocates and sets up the connection a single time.
///
/// `size_mb` represents the size of the shared memory mapping between the processes (in MB).
/// This function accepts sizes between 1 MB and 1 GB.
#[must_use = "the connection is closed on drop"]
pub fn to_child(size_mb: u32, command: &mut Command) -> Result<BridgeToChild, BridgeError> {
    if size_mb == 0 || size_mb > 1024 {
        return Err(BridgeError::Size(size_mb));
    }

    let size = size_of::<SharedBridge>() + size_mb as usize * 1024 * 1024;

    let file_mapping = FileMapping::open(size)?;
    let file_view = file_mapping.to_view()?;

    // SAFETY: `file_view` is a contiguous slice of shared memory.
    let shared =
        unsafe { SharedBridge::new_in(file_view.as_slice()).ok_or(BridgeError::Size(size_mb))? };

    serialize_into_command(&file_mapping, command);

    Ok(BridgeToChild {
        shared,
        _file_view: file_view,
        _file_mapping: file_mapping,
    })
}

impl BridgeToParent {
    /// Returns a writer implementing [`io::Write`] for sending [`MsgToParent::ConsoleLog`]
    /// messages.
    ///
    /// The writer will error if it receives invalid UTF-8.
    #[inline]
    pub fn console_log_writer(
        &self,
    ) -> LogWriter<impl Fn(Box<str>) -> MsgToParent + Clone + 'static> {
        LogWriter {
            bridge: self.clone(),
            to_msg: MsgToParent::ConsoleLog,
        }
    }

    /// Returns a writer implementing [`io::Write`] for sending [`MsgToParent::FileLog`]
    /// messages.
    ///
    /// The writer will error if it receives invalid UTF-8.
    #[inline]
    pub fn file_log_writer(&self) -> LogWriter<impl Fn(Box<str>) -> MsgToParent + Clone + 'static> {
        LogWriter {
            bridge: self.clone(),
            to_msg: MsgToParent::FileLog,
        }
    }

    /// Fulfill a RPC request from the parent process with the appropriate function.
    ///
    /// Sends a [`MsgToParent::Response`] to the parent process.
    pub fn fulfill<Req, F>(&self, (id, req): (RequestId, Request), f: F) -> Result<(), SendError>
    where
        Req: ConvertRequest + UnwindSafe,
        F: FnOnce(Req) -> Req::Res + UnwindSafe,
    {
        let res = req.fulfill(f).map(Req::Res::into_res);
        self.send(MsgToParent::Response((id, res)))?;
        Ok(())
    }

    /// Send a [`MsgToParent`] to the parent process.
    pub fn send(&self, msg: MsgToParent) -> Result<(), SendError> {
        self.shared.to_parent.send::<_, rancor::Error>(msg)
    }

    /// Receive messages from the parent process.
    ///
    /// Only one thread can be in this span at a time.
    pub fn enter_recv_span(
        &self,
    ) -> Result<RecvSpanGuard<'_, MsgToChild, rancor::Error>, SpanError> {
        self.shared.to_child.enter_recv_span()
    }
}

impl BridgeToChild {
    /// Queue a RPC request in the child process and block until it completes, yielding the result.
    ///
    /// Sends a [`MsgToChild::Request`] to the child process.
    pub fn request<Req>(&self, req: Req) -> Result<Req::Res, RequestError>
    where
        Req: ConvertRequest,
    {
        if self.shared.to_parent.is_current_thread_recv() {
            // Prevent awaiting a request from the receive loop as it will block the thread.
            return Err(RequestError::RequestFromRecv);
        }

        let req = req.into_req();
        let id = Request::generate_id();
        req.await_response(id, |(id, req)| self.send(MsgToChild::Request((id, req))))
    }

    /// Send a [`MsgToChild`] to the parent process.
    pub fn send(&self, msg: MsgToChild) -> Result<(), SendError> {
        self.shared.to_child.send(msg)
    }

    /// Receive messages from the child process.
    ///
    /// Only one thread can be in this span at a time.
    pub fn enter_recv_span(
        &self,
    ) -> Result<RecvSpanGuard<'_, MsgToParent, rancor::Error>, SpanError> {
        self.shared.to_parent.enter_recv_span()
    }
}

impl<F: Fn(Box<str>) -> MsgToParent + Clone> io::Write for LogWriter<F> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let str = str::from_utf8(buf).map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        self.bridge
            .send((self.to_msg)(str.into()))
            .map_err(io::Error::other)?;

        Ok(str.len())
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        // Could send `MsgToParent::Flush` here but `recv_loop` already does so automatically.
        Ok(())
    }
}

#[repr(C)]
#[derive(serde::Serialize, serde::Deserialize)]
struct FileMapping {
    handle: usize,
    size: usize,
}

#[repr(C)]
struct FileView {
    ptr: NonNull<c_void>,
    size: usize,
}

impl FileMapping {
    fn open(size: usize) -> Result<Self, WinError> {
        // `INHERIT_HANDLE` so the child process can use the handle directly.
        let handle = unsafe {
            CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                INHERIT_HANDLE,
                PAGE_READWRITE,
                (size >> 32) as u32,
                size as u32,
                PCWSTR::null(),
            )?
        };

        Ok(Self {
            handle: handle.0 as usize,
            size,
        })
    }

    fn to_view(&self) -> Result<FileView, WinError> {
        let size = self.size;

        // Non-null or bail.
        let ptr = unsafe {
            NonNull::new(MapViewOfFile(self.handle(), FILE_MAP_ALL_ACCESS, 0, 0, size).Value)
                .ok_or_else(WinError::from_thread)?
        };

        Ok(FileView { ptr, size })
    }

    fn handle(&self) -> HANDLE {
        HANDLE(self.handle as *mut c_void)
    }
}

impl FileView {
    fn as_slice(&self) -> NonNull<[u8]> {
        NonNull::slice_from_raw_parts(self.ptr.cast(), self.size)
    }
}

const INHERIT_HANDLE: Option<*const SECURITY_ATTRIBUTES> = Some(&SECURITY_ATTRIBUTES {
    nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
    lpSecurityDescriptor: std::ptr::null_mut(),
    bInheritHandle: BOOL(1),
});

impl EnvVars for FileMapping {
    const PREFIX: &'static str = "ME3_BRIDGE_FILE_";
}

impl EnvVars for &FileMapping {
    const PREFIX: &'static str = "ME3_BRIDGE_FILE_";
}

impl Drop for FileMapping {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle());
        }
    }
}

impl Drop for FileView {
    fn drop(&mut self) {
        unsafe {
            let _ = UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                Value: self.ptr.as_ptr(),
            });
        }
    }
}

unsafe impl Send for BridgeToParent {}

unsafe impl Sync for BridgeToParent {}

unsafe impl Send for BridgeToChild {}

unsafe impl Sync for BridgeToChild {}
