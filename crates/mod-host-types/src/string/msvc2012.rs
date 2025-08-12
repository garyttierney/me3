use std::ptr::{self, NonNull};

use rdvec::{
    alloc::{Alloc, AllocatorAware},
    RawVec,
};

use crate::{alloc::DlStdAllocator, string::RawCxxString};

#[repr(C)]
pub struct DlString<T: 'static, const E: u8> {
    pub raw: RawCxxString<T>,
    pub alloc: DlStdAllocator,
    pub encoding: u8,
}

impl<T, const E: u8> DlString<T, E> {
    #[inline]
    pub fn new_in(alloc: DlStdAllocator) -> Self {
        Self {
            raw: Default::default(),
            encoding: E,
            alloc,
        }
    }
}

unsafe impl<T, const E: u8> RawVec<T> for DlString<T, E> {
    #[inline]
    fn as_ptr(&self) -> *const T {
        self.raw.as_ptr()
    }

    #[inline]
    fn as_mut_ptr(&mut self) -> *mut T {
        self.raw.as_mut_ptr()
    }

    #[inline]
    fn len(&self) -> usize {
        self.raw.len()
    }

    #[inline]
    fn capacity(&self) -> usize {
        self.raw.capacity()
    }

    #[inline]
    fn max_len(&self) -> usize {
        self.raw.max_len()
    }

    #[inline]
    unsafe fn set_len(&mut self, new_len: usize) {
        unsafe {
            self.raw.set_len(new_len);
        }
    }

    #[inline]
    unsafe fn set_buf(&mut self, new_buf: *mut [T]) {
        unsafe {
            self.raw.set_buf(new_buf);
        }
    }
}

impl<T, const E: u8> Alloc<T> for DlString<T, E> {
    fn alloc(&self, count: usize) -> rdvec::alloc::Result<NonNull<[T]>> {
        if count > RawCxxString::<T>::small_mode_capacity() {
            self.alloc.alloc(count + 1)
        } else {
            let inner_buf = unsafe { self.raw.inner.buf.align_to::<T>().1 };
            Ok(inner_buf.into())
        }
    }

    unsafe fn dealloc(&self, ptr: NonNull<[T]>) -> rdvec::alloc::Result<()> {
        unsafe {
            if ptr::addr_eq(ptr.as_ptr(), &raw const self.raw.inner.buf) {
                return Ok(());
            }

            self.alloc.dealloc(NonNull::slice_from_raw_parts(
                ptr.cast::<T>(),
                ptr.len() + 1,
            ))
        }
    }
}

impl<T: 'static, const E: u8> AllocatorAware<T> for DlString<T, E> {
    type Alloc = dyn Alloc<T>;

    #[inline]
    fn allocator(&self) -> &Self::Alloc {
        self
    }
}
