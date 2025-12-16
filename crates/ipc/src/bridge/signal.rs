use std::sync::atomic::{fence, AtomicBool, AtomicU32, Ordering};

use windows::{
    core::{Error as WinError, PCWSTR},
    Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0},
        System::Threading::{CreateEventW, ResetEvent, SetEvent, WaitForSingleObject, INFINITE},
    },
};

use crate::bridge::INHERIT_HANDLE;

#[repr(C, align(64))]
pub struct MpscSignal {
    signaled: AtomicBool,
    handle: HANDLE,
}

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
            return Ok(());
        }
        self.signaled.store(true, Ordering::Release);
        unsafe { SetEvent(self.handle) }
    }

    #[inline]
    pub fn wait(&self) -> Result<(), WinError> {
        if self.signaled.swap(false, Ordering::Acquire) {
            self.signaled.store(false, Ordering::Release);
            return Ok(());
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
        if self.asleep.fetch_sub(1, Ordering::Release) != 0 {
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
