use std::sync::atomic::{fence, AtomicBool, AtomicU32, Ordering};

use windows::{
    core::{Error as WinError, PCWSTR},
    Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0},
        System::Threading::{CreateEventW, ResetEvent, SetEvent, WaitForSingleObject, INFINITE},
    },
};

use crate::bridge::INHERIT_HANDLE;

/// Multiple producers, single consumer signal.
///
/// Misusing it (having multiple consumers) is not unsafe but may lead to deadlocks.
#[repr(C, align(64))]
pub struct MpscSignal {
    signaled: AtomicBool,
    handle: HANDLE,
}

/// Single producer, multiple consumers signal.
///
/// Misusing it (having multiple producers) is not unsafe but may lead to deadlocks.
#[repr(C, align(64))]
pub struct SpmcSignal {
    asleep: AtomicU32,
    handle: HANDLE,
}

impl MpscSignal {
    #[inline]
    pub fn new() -> Self {
        let handle = unsafe {
            CreateEventW(INHERIT_HANDLE, false, false, PCWSTR::null()).expect("CreateEventW failed")
        };

        Self {
            signaled: AtomicBool::new(false),
            handle,
        }
    }

    #[inline]
    pub fn notify(&self) -> Result<(), WinError> {
        if self.signaled.load(Ordering::Acquire) {
            // Can get away with this because we only ever expect a single thread
            // to set this to `false`, so we can elide future OS calls.
            return Ok(());
        }
        self.signaled.store(true, Ordering::Release);
        unsafe { SetEvent(self.handle) }
    }

    #[inline]
    pub fn wait(&self) -> Result<(), WinError> {
        if self.signaled.swap(false, Ordering::AcqRel) {
            // Was signaled, reset the event to prevent spurious wakeups.
            return unsafe { ResetEvent(self.handle) };
        }
        match unsafe { WaitForSingleObject(self.handle, INFINITE) } {
            WAIT_OBJECT_0 => Ok(()),
            _ => Err(WinError::from_win32()),
        }
    }
}

impl SpmcSignal {
    #[inline]
    pub fn new() -> Self {
        // This event is manual reset since we want to wake up multiple threads.
        let handle = unsafe {
            CreateEventW(INHERIT_HANDLE, true, false, PCWSTR::null()).expect("CreateEventW failed")
        };

        Self {
            asleep: AtomicU32::new(0),
            handle,
        }
    }

    #[inline]
    pub fn notify(&self) -> Result<(), WinError> {
        if self.asleep.load(Ordering::Acquire) == 0 {
            // No threads are sleeping
            return Ok(());
        }
        unsafe { SetEvent(self.handle) }
    }

    #[inline]
    pub fn wait(&self) -> Result<(), WinError> {
        let _ = self.asleep.fetch_add(1, Ordering::Relaxed);
        if unsafe { WaitForSingleObject(self.handle, INFINITE) != WAIT_OBJECT_0 } {
            return Err(WinError::from_win32());
        }
        if self.asleep.fetch_sub(1, Ordering::Release) != 1 {
            // Other threads are sleeping.
            return Ok(());
        }
        fence(Ordering::Acquire);
        unsafe { ResetEvent(self.handle) }
    }
}

impl Drop for MpscSignal {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

impl Drop for SpmcSignal {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

unsafe impl Send for MpscSignal {}

unsafe impl Sync for MpscSignal {}

unsafe impl Send for SpmcSignal {}

unsafe impl Sync for SpmcSignal {}
