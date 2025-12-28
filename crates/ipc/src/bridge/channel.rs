use std::{
    hash::BuildHasher,
    marker::PhantomData,
    ptr::NonNull,
    sync::atomic::{AtomicU64, Ordering},
};

use rkyv::{
    api::high::{access, deserialize, HighDeserializer, HighSerializer, HighValidator},
    bytecheck::CheckBytes,
    rancor::{self, Source},
    ser::allocator::ArenaHandle,
    to_bytes,
    util::AlignedVec,
    Archive, Deserialize, Serialize,
};
use windows::core::Error as WinError;

use crate::{
    bridge::{
        buffer::{BipBuffer, WriteError},
        signal::{MpscSignal, SpmcSignal},
    },
    identity_hasher::IdentityBuildHasher,
};

#[repr(C)]
pub struct Channel {
    queue: BipBuffer,
    has_messages: MpscSignal,
    has_capacity: SpmcSignal,
    recv_thread_id: AtomicU64,
}

pub struct RecvSpanGuard<'a, T, E> {
    channel: &'a Channel,
    _marker: PhantomData<Result<T, E>>,
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
pub enum RecvError<E = rancor::Error> {
    #[error(transparent)]
    Os(#[from] WinError),

    #[error("failed to deserialize message: {0}")]
    Deserialize(E),
}

#[derive(Debug, thiserror::Error)]
#[error("another thread has already entered this span")]
pub struct SpanError;

impl Channel {
    pub fn new() -> Self {
        Self {
            queue: BipBuffer::new(),
            has_messages: MpscSignal::new(),
            has_capacity: SpmcSignal::new(),
            recv_thread_id: AtomicU64::new(0),
        }
    }

    /// # Safety
    ///
    /// `buf` must be a pointer within the same allocation as `self`.
    ///
    /// # Panics
    ///
    /// If `buf` is more than 4 gigabytes away from `self`.
    pub unsafe fn init(&mut self, buf: NonNull<u8>, len: u32) {
        unsafe {
            self.queue.init(buf, len);
        }
    }

    pub fn send<M, E>(&self, msg: M) -> Result<(), SendError<E>>
    where
        M: for<'a> Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, E>>,
        E: Source,
    {
        // Push the message and let the receiver know.
        self.push_msg(msg)?;
        self.has_messages.notify()?;
        Ok(())
    }

    /// Try to enter the single threaded recv span, which returns an error if
    /// another thread has already entered.
    pub fn enter_recv_span<T, E>(&self) -> Result<RecvSpanGuard<'_, T, E>, SpanError>
    where
        T: Archive,
        T::Archived: Deserialize<T, HighDeserializer<E>> + for<'a> CheckBytes<HighValidator<'a, E>>,
        E: Source,
    {
        // `current_thread_id` returns a nonzero value, so 0 can be the sentinel.
        let id = current_thread_id();
        match self
            .recv_thread_id
            .compare_exchange(0, id, Ordering::Acquire, Ordering::Relaxed)
        {
            Ok(_) => Ok(RecvSpanGuard {
                channel: self,
                _marker: PhantomData,
            }),
            Err(_) => Err(SpanError),
        }
    }

    /// Checks if the current thread is within the recv span.
    pub fn is_current_thread_recv(&self) -> bool {
        current_thread_id() == self.recv_thread_id.load(Ordering::Relaxed)
    }

    /// # Safety
    ///
    /// This function can only be called by one thread at a time. See [`RecvSpanGuard::recv`].
    unsafe fn recv<T, E>(&self) -> Result<T, RecvError<E>>
    where
        T: Archive,
        T::Archived: Deserialize<T, HighDeserializer<E>> + for<'a> CheckBytes<HighValidator<'a, E>>,
        E: Source,
    {
        let message = loop {
            self.has_messages.wait()?;

            // SAFETY: upheld by caller.
            let result = unsafe {
                self.queue.read(|bytes| {
                    let archived = access::<T::Archived, E>(bytes)?;
                    deserialize::<T, E>(archived)
                })
            };

            if let Some(result) = result {
                break result.map_err(RecvError::Deserialize);
            }
        };

        self.has_capacity.notify()?;

        message
    }

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

impl<T, E> RecvSpanGuard<'_, T, E>
where
    T: Archive,
    T::Archived: Deserialize<T, HighDeserializer<E>> + for<'a> CheckBytes<HighValidator<'a, E>>,
    E: Source,
{
    pub fn recv(&self) -> Result<T, RecvError<E>> {
        // SAFETY: this guard ensures singlethreaded access to the channel.
        unsafe { self.channel.recv() }
    }
}

fn current_thread_id() -> u64 {
    IdentityBuildHasher.hash_one(std::thread::current().id())
}

impl<T, E> Drop for RecvSpanGuard<'_, T, E> {
    fn drop(&mut self) {
        // Another thread can enter the span.
        self.channel.recv_thread_id.store(0, Ordering::Release);
    }
}
