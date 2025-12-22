use std::{hint, ptr::NonNull};

use rkyv::{
    api::high::{access, HighSerializer, HighValidator},
    bytecheck::CheckBytes,
    rancor::{self, Source},
    ser::allocator::ArenaHandle,
    to_bytes,
    util::AlignedVec,
    Portable, Serialize,
};
use windows::core::Error as WinError;

use crate::bridge::{
    buffer::{BipBuffer, WriteError},
    guard::{SpGuardError, SpSpan},
    signal::{MpscSignal, SpmcSignal},
};

#[repr(C)]
pub struct Channel {
    pub(super) queue: BipBuffer,
    pub(super) has_messages: MpscSignal,
    pub(super) has_capacity: SpmcSignal,
}

#[derive(Debug, thiserror::Error)]
pub enum SendError<E = rancor::Error> {
    #[error(transparent)]
    Os(#[from] WinError),

    #[error("failed to send message: {0}")]
    Write(#[from] WriteError),

    #[error("failed to serialize message: {0}")]
    Serialize(E),
}

#[derive(Debug, thiserror::Error)]
pub enum RecvError {
    #[error(transparent)]
    Os(#[from] WinError),

    #[error(transparent)]
    Producer(#[from] SpGuardError),
}

impl Channel {
    const SPIN_COUNT: u32 = 1000;

    #[inline]
    pub fn new() -> Self {
        Self {
            queue: BipBuffer::new(),
            has_messages: MpscSignal::new(),
            has_capacity: SpmcSignal::new(),
        }
    }

    pub unsafe fn init(&mut self, buf: NonNull<u8>, len: u32) {
        unsafe {
            self.queue.init(buf, len);
        }
    }

    #[inline]
    pub fn send<M, E>(&self, msg: M) -> Result<(), SendError<E>>
    where
        M: for<'a> Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, E>>,
        E: Source,
    {
        // Push the message and let the receiver know.
        self.push_msg(msg)?;
        self.has_messages.notify().map_err(SendError::Os)
    }

    #[inline]
    pub fn recv_loop<F, A, E>(&self, mut f: F, flush: Option<A>) -> Result<(), RecvError>
    where
        F: FnMut(Result<&A, E>),
        A: Portable + for<'a> CheckBytes<HighValidator<'a, E>>,
        E: Source,
    {
        static SP_SPAN: SpSpan = SpSpan::new();
        let _guard = SP_SPAN.enter()?;

        loop {
            // Spin, receiving messages.
            for _ in 0..Self::SPIN_COUNT {
                // SAFETY: only one thread can call `queue.read()` because of the guard.
                while unsafe { self.queue.read(|bytes| f(access(bytes))).is_some() } {}
                hint::spin_loop();
            }

            // There may be senders waiting for space in the queue.
            self.has_capacity.notify()?;

            // Flush, since we've gone through all the messages for now.
            if let Some(flush) = &flush {
                f(Ok(flush));
            }

            // Wait for more messages.
            self.has_messages.wait()?;
        }
    }

    #[inline]
    fn push_msg<M, E>(&self, msg: M) -> Result<(), SendError<E>>
    where
        M: for<'a> Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, E>>,
        E: Source,
    {
        let bytes = to_bytes::<E>(&msg).map_err(SendError::Serialize)?;

        // Block until the message can be written.
        loop {
            match self.queue.write(&bytes) {
                Ok(()) => return Ok(()),
                Err(WriteError::Full) => self.has_capacity.wait()?,
                Err(e) => return Err(e.into()),
            }
        }
    }
}
