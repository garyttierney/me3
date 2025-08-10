use rdvec::{alloc::AllocatorAware, RawVec};

use crate::{alloc::DlStdAllocator, vector::RawCxxVec};

#[repr(C)]
pub struct DlVector<T> {
    pub alloc: DlStdAllocator,
    pub raw: RawCxxVec<T>,
}

impl<T> DlVector<T> {
    pub fn new_in(alloc: DlStdAllocator) -> Self {
        Self {
            raw: Default::default(),
            alloc,
        }
    }
}

unsafe impl<T> RawVec<T> for DlVector<T> {
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

impl<T> AllocatorAware<T> for DlVector<T> {
    type Alloc = DlStdAllocator;

    #[inline]
    fn allocator(&self) -> &Self::Alloc {
        &self.alloc
    }
}
