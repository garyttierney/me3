use std::sync::atomic::{fence, AtomicU32, Ordering};

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
#[repr(C, align(128))]
pub struct MpscSignal {
    notify_count: AtomicU32,
    handle: HANDLE,
}

/// Single producer, multiple consumers signal.
///
/// Misusing it (having multiple producers) is not unsafe but may lead to deadlocks.
#[repr(C, align(128))]
pub struct SpmcSignal {
    asleep: AtomicU32,
    handle: HANDLE,
}

impl MpscSignal {
    pub fn new() -> Self {
        let handle = unsafe {
            CreateEventW(INHERIT_HANDLE, false, false, PCWSTR::null()).expect("CreateEventW failed")
        };

        Self {
            notify_count: AtomicU32::new(1),
            handle,
        }
    }

    pub fn notify(&self) -> Result<(), WinError> {
        if self.notify_count.fetch_add(1, Ordering::AcqRel) != 0 {
            return Ok(());
        }
        unsafe { SetEvent(self.handle) }
    }

    pub fn wait(&self) -> Result<(), WinError> {
        if self.notify_count.fetch_sub(1, Ordering::AcqRel) != 1 {
            return Ok(());
        }
        match unsafe { WaitForSingleObject(self.handle, INFINITE) } {
            WAIT_OBJECT_0 => Ok(()),
            _ => Err(WinError::from_thread()),
        }
    }
}

impl SpmcSignal {
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

    pub fn notify(&self) -> Result<(), WinError> {
        if self.asleep.load(Ordering::Acquire) == 0 {
            // No threads are sleeping
            return Ok(());
        }
        unsafe { SetEvent(self.handle) }
    }

    pub fn wait(&self) -> Result<(), WinError> {
        let _ = self.asleep.fetch_add(1, Ordering::Relaxed);
        if unsafe { WaitForSingleObject(self.handle, INFINITE) != WAIT_OBJECT_0 } {
            return Err(WinError::from_thread());
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
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

impl Drop for SpmcSignal {
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
