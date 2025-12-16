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
    buffer::BipBuffer,
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
            for _ in 0..Self::SPIN_COUNT {
                while unsafe { self.queue.read(|bytes| f(access(bytes))).is_some() } {}
                hint::spin_loop();
            }

            self.has_capacity.notify()?;

            if let Some(flush) = &flush {
                f(Ok(flush));
            }

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

        loop {
            if self.queue.write(&bytes).is_ok() {
                break;
            }
            self.has_capacity.wait()?;
        }

        Ok(())
    }
}
