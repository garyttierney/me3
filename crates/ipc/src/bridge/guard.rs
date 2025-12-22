use std::{
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
};

/// Span that may only be entered by a single thread.
pub struct RecvLoopSpan {
    entered_thread_id: AtomicU64,
}

pub struct RecvLoopSpanGuard<'a>(&'a AtomicU64);

#[derive(Debug, thiserror::Error)]
#[error("another thread has already entered this span")]
pub struct GuardError;

impl RecvLoopSpan {
    pub const fn new() -> Self {
        Self {
            entered_thread_id: AtomicU64::new(0),
        }
    }

    /// Try to enter the single threaded recv loop span, which returns an error if
    /// another thread has already entered.
    #[inline]
    pub fn enter(&self) -> Result<RecvLoopSpanGuard<'_>, GuardError> {
        // `current_thread_id` returns a nonzero value, so 0 can be the sentinel.
        let id = Self::current_thread_id().get();
        match self
            .entered_thread_id
            .compare_exchange(0, id, Ordering::Acquire, Ordering::Relaxed)
        {
            Ok(_) => Ok(RecvLoopSpanGuard(&self.entered_thread_id)),
            Err(_) => Err(GuardError),
        }
    }

    /// Checks if the current thread is within the recv loop span.
    #[inline]
    pub fn is_entered_by_current_thread(&self) -> bool {
        Self::current_thread_id().get() == self.entered_thread_id.load(Ordering::Relaxed)
    }

    #[inline]
    fn current_thread_id() -> NonZeroU64 {
        std::thread::current().id().as_u64()
    }
}

impl Drop for RecvLoopSpanGuard<'_> {
    #[inline]
    fn drop(&mut self) {
        // Another thread can enter the span.
        self.0.store(0, Ordering::Release);
    }
}
