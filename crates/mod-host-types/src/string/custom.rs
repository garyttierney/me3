use std::ptr::NonNull;

use rdvec::{
    alloc::{Alloc, AllocatorAware},
    RawVec, Vec,
};

use crate::{alloc::DlStdAllocator, string::DlCharacterSet};

#[repr(C)]
pub struct DlCustomUtf16Str {
    inner: DlRawUtf16Str,
    buf_or_cap: DlCustomUtf16StrUnion,
    alloc: DlStdAllocator,
}

#[repr(C)]
union DlCustomUtf16StrUnion {
    buf: [u16; 4],
    cap: usize,
}

#[repr(C)]
struct DlRawUtf16Str {
    _vtable: usize,
    ptr: *mut u16,
    len: usize,
    _unk18: u32,
    char_size: u16,
    encoding: DlCharacterSet,
    flags: u8,
}

impl DlCustomUtf16Str {
    const SMALL_MODE_CAP: usize = 4 - 1;

    fn new() -> Self {
        let alloc = DlStdAllocator::new();

        // Unambiguously non-inline allocation, never self-referential.
        let ptr: NonNull<[u16]> = alloc.alloc(Self::SMALL_MODE_CAP + 1 + 1).unwrap();
        let cap = ptr.len() - 1;

        Self {
            inner: DlRawUtf16Str {
                _vtable: 0,
                ptr: ptr.as_ptr() as *mut u16,
                len: 0,
                _unk18: 0,
                char_size: 2,
                encoding: DlCharacterSet::Utf16,
                flags: 0,
            },
            buf_or_cap: DlCustomUtf16StrUnion { cap },
            alloc,
        }
    }

    fn is_small_mode(&self) -> bool {
        unsafe { self.inner.ptr.cast_const() == self.buf_or_cap.buf.as_ptr() }
    }
}

impl Drop for DlCustomUtf16Str {
    fn drop(&mut self) {
        let Some(ptr) = NonNull::new(self.as_mut_ptr()) else {
            return;
        };

        unsafe {
            let _ = self.dealloc(NonNull::slice_from_raw_parts(ptr, self.capacity()));
        }
    }
}

unsafe impl RawVec<u16> for DlCustomUtf16Str {
    fn as_ptr(&self) -> *const u16 {
        if self.is_small_mode() {
            unsafe { self.buf_or_cap.buf.as_ptr() }
        } else {
            self.inner.ptr
        }
    }

    fn as_mut_ptr(&mut self) -> *mut u16 {
        if self.is_small_mode() {
            unsafe { self.buf_or_cap.buf.as_mut_ptr() }
        } else {
            self.inner.ptr
        }
    }

    fn len(&self) -> usize {
        self.inner.len
    }

    fn capacity(&self) -> usize {
        if self.is_small_mode() {
            Self::SMALL_MODE_CAP
        } else {
            unsafe { self.buf_or_cap.cap }
        }
    }

    fn max_len(&self) -> usize {
        isize::MAX as usize / 2 - 1
    }

    unsafe fn set_len(&mut self, new_len: usize) {
        // Write the nul terminator.
        unsafe {
            self.as_mut_ptr().add(new_len).write_bytes(0, 1);
        }

        self.inner.len = new_len;
    }

    unsafe fn set_buf(&mut self, new_buf: *mut [u16]) {
        self.buf_or_cap.cap = new_buf.len() - 1;

        // Write the nul terminator.
        unsafe {
            (new_buf as *mut u16).add(self.len()).write_bytes(0, 1);
        }

        self.inner.ptr = new_buf as *mut u16;
    }
}

impl Alloc<u16> for DlCustomUtf16Str {
    fn alloc(&self, count: usize) -> rdvec::alloc::Result<NonNull<[u16]>> {
        // Unambiguously non-inline allocation, never self-referential.
        self.alloc.alloc(count.max(Self::SMALL_MODE_CAP + 1) + 1)
    }

    unsafe fn dealloc(&self, ptr: NonNull<[u16]>) -> rdvec::alloc::Result<()> {
        unsafe {
            if self.is_small_mode() {
                return Ok(());
            }

            self.alloc.dealloc(NonNull::slice_from_raw_parts(
                ptr.cast::<u16>(),
                ptr.len() + 1,
            ))
        }
    }
}

impl AllocatorAware<u16> for DlCustomUtf16Str {
    type Alloc = dyn Alloc<u16>;

    fn allocator(&self) -> &Self::Alloc {
        self
    }
}

impl PartialEq for DlCustomUtf16Str {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice().eq(other.as_slice())
    }
}

impl PartialOrd for DlCustomUtf16Str {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for DlCustomUtf16Str {}

impl Ord for DlCustomUtf16Str {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}

impl From<&str> for DlCustomUtf16Str {
    fn from(str: &str) -> Self {
        let mut encoded = Self::new();
        for char in str.encode_utf16() {
            encoded.push(char);
        }
        encoded
    }
}
