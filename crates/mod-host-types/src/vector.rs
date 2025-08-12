use std::{
    borrow::{Borrow, BorrowMut},
    fmt,
    mem::{self, ManuallyDrop},
    ops::{Deref, DerefMut},
    ptr,
};

use me3_mod_protocol::Game;
use rdvec::{
    alloc::{Alloc, AllocatorAware},
    RawVec, Vec,
};

use crate::{alloc::DlStdAllocator, game::GAME};

mod msvc2012;
mod msvc2015;

#[repr(C)]
struct RawCxxVec<T> {
    first: *mut T,
    last: *mut T,
    end: *mut T,
}

#[repr(C)]
pub union DlVector<T> {
    msvc2012: ManuallyDrop<msvc2012::DlVector<T>>,
    msvc2015: ManuallyDrop<msvc2015::DlVector<T>>,
}

impl<T> DlVector<T> {
    #[inline]
    pub fn new() -> Self {
        Self::new_in(Default::default())
    }

    #[inline]
    pub fn new_in(alloc: DlStdAllocator) -> Self {
        match *GAME {
            game if game < Game::Sekiro => Self {
                msvc2012: ManuallyDrop::new(msvc2012::DlVector::new_in(alloc)),
            },
            _ => Self {
                msvc2015: ManuallyDrop::new(msvc2015::DlVector::new_in(alloc)),
            },
        }
    }

    /// Reads and verifies the storage bounds pointers and the allocator address:
    /// `(first, last, end, alloc_addr)`
    ///
    /// # Safety
    ///
    /// `ptr` must be valid for reads of up to 32 bytes.
    #[inline]
    pub unsafe fn try_read_raw_parts(ptr: *const Self) -> Option<(*mut T, *mut T, *mut T, usize)> {
        if ptr.is_null() || !ptr.is_aligned() {
            return None;
        }

        let (RawCxxVec { first, last, end }, alloc_addr) = match *GAME {
            game if game < Game::Sekiro => unsafe {
                let ptr = (&raw const (*ptr).msvc2012) as *const msvc2012::DlVector<T>;

                (
                    ptr::read(&raw const (*ptr).raw),
                    ptr::read(&raw const (*ptr).alloc as *const usize),
                )
            },
            _ => unsafe {
                let ptr = (&raw const (*ptr).msvc2015) as *const msvc2015::DlVector<T>;

                (
                    ptr::read(&raw const (*ptr).raw),
                    ptr::read(&raw const (*ptr).alloc as *const usize),
                )
            },
        };

        (alloc_addr.is_multiple_of(8)
            && first.is_aligned()
            && last.is_aligned()
            && end.is_aligned()
            && first <= last
            && last <= end)
            .then_some((first, last, end, alloc_addr))
    }

    #[inline]
    fn as_dyn(&self) -> &dyn Vec<T, Alloc = DlStdAllocator> {
        match *GAME {
            game if game < Game::Sekiro => unsafe { &*self.msvc2012 },
            _ => unsafe { &*self.msvc2015 },
        }
    }

    #[inline]
    fn as_mut_dyn(&mut self) -> &mut dyn Vec<T, Alloc = DlStdAllocator> {
        match *GAME {
            game if game < Game::Sekiro => unsafe { &mut *self.msvc2012 },
            _ => unsafe { &mut *self.msvc2015 },
        }
    }
}

unsafe impl<T> RawVec<T> for RawCxxVec<T> {
    #[inline]
    fn as_ptr(&self) -> *const T {
        self.first
    }

    #[inline]
    fn as_mut_ptr(&mut self) -> *mut T {
        self.first
    }

    #[inline]
    fn len(&self) -> usize {
        if mem::size_of::<T>() != 0 {
            unsafe { self.last.offset_from_unsigned(self.first) }
        } else {
            unsafe { self.last.byte_offset_from_unsigned(self.first) }
        }
    }

    #[inline]
    fn capacity(&self) -> usize {
        if mem::size_of::<T>() != 0 {
            unsafe { self.end.offset_from_unsigned(self.first) }
        } else {
            unsafe { self.end.byte_offset_from_unsigned(self.first) }
        }
    }

    #[inline]
    fn max_len(&self) -> usize {
        isize::MAX as usize / mem::size_of::<T>().min(1)
    }

    #[inline]
    unsafe fn set_len(&mut self, new_len: usize) {
        let capacity = self.capacity();

        if new_len > capacity {
            panic!("new length (is {new_len}) exceeds capacity (is {capacity})");
        }

        if mem::size_of::<T>() != 0 {
            self.last = self.first.wrapping_add(new_len);
        } else {
            self.last = self.first.wrapping_byte_add(new_len);
        }
    }

    #[inline]
    unsafe fn set_buf(&mut self, new_buf: *mut [T]) {
        if new_buf.is_empty() {
            *self = Self::default();
            return;
        }

        let len = self.len();
        let new_capacity = new_buf.len();

        if len > new_capacity {
            panic!("length (is {len}) exceeds new capacity (is {new_capacity})");
        }

        self.first = new_buf as *mut T;

        if mem::size_of::<T>() != 0 {
            self.last = self.first.wrapping_add(len);
            self.end = self.first.wrapping_add(new_capacity);
        } else {
            self.last = self.first.wrapping_byte_add(len);
            self.end = self.first.wrapping_byte_add(new_capacity);
        }
    }
}

unsafe impl<T> RawVec<T> for DlVector<T> {
    #[inline]
    fn as_ptr(&self) -> *const T {
        self.as_dyn().as_ptr()
    }

    #[inline]
    fn as_mut_ptr(&mut self) -> *mut T {
        self.as_mut_dyn().as_mut_ptr()
    }

    #[inline]
    fn len(&self) -> usize {
        self.as_dyn().len()
    }

    #[inline]
    fn capacity(&self) -> usize {
        self.as_dyn().capacity()
    }

    #[inline]
    fn max_len(&self) -> usize {
        self.as_dyn().max_len()
    }

    #[inline]
    unsafe fn set_len(&mut self, new_len: usize) {
        unsafe {
            self.as_mut_dyn().set_len(new_len);
        }
    }

    #[inline]
    unsafe fn set_buf(&mut self, new_buf: *mut [T]) {
        unsafe {
            self.as_mut_dyn().set_buf(new_buf);
        }
    }
}

impl<T> AllocatorAware<T> for DlVector<T> {
    type Alloc = DlStdAllocator;

    #[inline]
    fn allocator(&self) -> &Self::Alloc {
        self.as_dyn().allocator()
    }
}

impl<T> AsRef<[T]> for DlVector<T> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> AsMut<[T]> for DlVector<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T> Borrow<[T]> for DlVector<T> {
    #[inline]
    fn borrow(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> BorrowMut<[T]> for DlVector<T> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T> Clone for DlVector<T>
where
    T: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        let mut cloned = Self::new_in(*self.allocator());

        cloned.extend_from_slice(self.as_slice());

        cloned
    }
}

impl<T> Deref for DlVector<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T> DerefMut for DlVector<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T> fmt::Debug for DlVector<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T> Default for RawCxxVec<T> {
    #[inline]
    fn default() -> Self {
        Self {
            first: ptr::null_mut(),
            last: ptr::null_mut(),
            end: ptr::null_mut(),
        }
    }
}

impl<T> Default for DlVector<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for DlVector<T> {
    fn drop(&mut self) {
        unsafe {
            let buf = self.as_buf();
            let elems = &raw mut *self.as_mut_slice();

            match *GAME {
                game if game < Game::Sekiro => self.msvc2012.raw = Default::default(),
                _ => self.msvc2015.raw = Default::default(),
            }

            elems.drop_in_place();
            self.allocator().dealloc(buf).unwrap();
        }
    }
}

unsafe impl<T> Send for DlVector<T> where T: Send {}

unsafe impl<T> Sync for DlVector<T> where T: Sync {}
