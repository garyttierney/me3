use std::{cell::UnsafeCell, slice};

use pelite::pe::{Pe, PeObject, PeView};
use windows::{core::PCWSTR, Win32::System::LibraryLoader::GetModuleHandleW};

#[derive(Clone, Copy)]
pub struct Executable {
    inner: &'static UnsafeCell<[u8]>,
}

impl Executable {
    /// # Safety
    ///
    /// Module must be accessed exclusively for the duration of the call
    /// (typically only possible in a suspended process).
    ///
    /// # Panics
    ///
    /// if `GetModuleHandleW(NULL)` fails.
    pub unsafe fn new() -> Self {
        unsafe {
            let view = PeView::module(GetModuleHandleW(PCWSTR::null()).unwrap().0 as _);
            let image = view.image();

            Self {
                inner: UnsafeCell::from_mut(slice::from_raw_parts_mut(
                    image.as_ptr() as _,
                    image.len(),
                )),
            }
        }
    }
}

unsafe impl PeObject<'static> for Executable {
    fn image(&self) -> &'static [u8] {
        unsafe { &*self.inner.get() }
    }

    fn align(&self) -> pelite::Align {
        pelite::Align::Section
    }
}

unsafe impl Pe<'static> for Executable {}

unsafe impl Send for Executable {}

unsafe impl Sync for Executable {}
