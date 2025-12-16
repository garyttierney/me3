use std::sync::atomic::{AtomicBool, Ordering};

pub struct SpSpan(AtomicBool);

pub struct SpGuard<'a>(&'a AtomicBool);

#[derive(Debug, thiserror::Error)]
#[error("another thread has already entered this span")]
pub struct SpGuardError;

impl SpSpan {
    pub const fn new() -> Self {
        Self(AtomicBool::new(false))
    }

    #[inline]
    pub fn enter(&self) -> Result<SpGuard<'_>, SpGuardError> {
        match self
            .0
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        {
            Ok(_) => Ok(SpGuard(&self.0)),
            Err(_) => Err(SpGuardError),
        }
    }
}

impl Drop for SpGuard<'_> {
    #[inline]
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
