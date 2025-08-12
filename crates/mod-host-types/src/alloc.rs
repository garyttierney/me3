//! `DLKR::DlAllocator` layout and compatible allocator for use in place of [`DlStdAllocator`]
//! from the `Dantelion2` in-house FromSoftware library.

use std::{
    alloc::{GlobalAlloc, Layout},
    mem::ManuallyDrop,
    ptr::NonNull,
};

use default::DEFAULT_DLALLOC;

mod default;

/// Commonly used polymorphic `DlAllocator` adapter for objects and containers.
///
/// Contains a pointer to a `DlAllocator` interface and implements [`GlobalAlloc`].
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DlStdAllocator {
    inner: NonNull<DlAllocator>,
}

#[repr(C)]
struct DlAllocator {
    vtable: NonNull<DlAllocatorVtable>,
}

#[repr(u32)]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default)]
enum DlHeapDirection {
    #[default]
    Front = 0,
    Back = 1,
}

#[repr(C)]
struct DlAllocatorVtable {
    dtor: extern "C" fn(this: NonNull<ManuallyDrop<DlAllocator>>),

    heap_id: extern "C" fn(this: NonNull<DlAllocator>) -> u32,

    allocator_id: extern "C" fn(this: NonNull<DlAllocator>) -> u32,

    capability: extern "C" fn(
        this: NonNull<DlAllocator>,
        out: NonNull<u32>,
        heap: DlHeapDirection,
    ) -> NonNull<u32>,

    total_size: extern "C" fn(this: NonNull<DlAllocator>) -> usize,

    free_size: extern "C" fn(this: NonNull<DlAllocator>) -> usize,

    max_size: extern "C" fn(this: NonNull<DlAllocator>) -> usize,

    num_blocks: extern "C" fn(this: NonNull<DlAllocator>) -> usize,

    block_size: extern "C" fn(this: NonNull<DlAllocator>, block: *mut u8) -> usize,

    allocate: extern "C" fn(this: NonNull<DlAllocator>, size: usize) -> *mut u8,

    allocate_aligned:
        extern "C" fn(this: NonNull<DlAllocator>, size: usize, alignment: usize) -> *mut u8,

    reallocate: extern "C" fn(this: NonNull<DlAllocator>, old: *mut u8, new_size: usize) -> *mut u8,

    reallocate_aligned: extern "C" fn(
        this: NonNull<DlAllocator>,
        old: *mut u8,
        new_size: usize,
        alignment: usize,
    ) -> *mut u8,

    free: extern "C" fn(this: NonNull<DlAllocator>, ptr: *mut u8),

    free_all: extern "C" fn(this: NonNull<DlAllocator>),

    back_allocate: extern "C" fn(this: NonNull<DlAllocator>, size: usize) -> *mut u8,

    back_allocate_aligned:
        extern "C" fn(this: NonNull<DlAllocator>, size: usize, alignment: usize) -> *mut u8,

    back_reallocate:
        extern "C" fn(this: NonNull<DlAllocator>, old: *mut u8, new_size: usize) -> *mut u8,

    back_reallocate_aligned: extern "C" fn(
        this: NonNull<DlAllocator>,
        old: *mut u8,
        new_size: usize,
        alignment: usize,
    ) -> *mut u8,

    back_free: extern "C" fn(this: NonNull<DlAllocator>, ptr: *mut u8),

    self_diagnose: extern "C" fn(this: NonNull<DlAllocator>) -> bool,

    is_valid_block: extern "C" fn(this: NonNull<DlAllocator>, block: *mut u8) -> bool,

    lock: extern "C" fn(this: NonNull<DlAllocator>),

    unlock: extern "C" fn(this: NonNull<DlAllocator>),

    block_of: extern "C" fn(this: NonNull<DlAllocator>, ptr: *mut u8) -> *mut u8,
}

impl DlStdAllocator {
    pub const fn new() -> Self {
        Self {
            inner: NonNull::from_ref(&DEFAULT_DLALLOC),
        }
    }

    pub fn addr(self) -> usize {
        self.inner.addr().get()
    }
}

unsafe impl GlobalAlloc for DlStdAllocator {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let alignment = layout.align();

        let vtable = unsafe { self.inner.as_ref().vtable.as_ref() };
        (vtable.allocate_aligned)(self.inner, size, alignment)
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) {
        let vtable = unsafe { self.inner.as_ref().vtable.as_ref() };
        (vtable.free)(self.inner, ptr);
    }
}

impl Default for DlStdAllocator {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for DlAllocator {}

unsafe impl Sync for DlAllocator {}

unsafe impl Send for DlStdAllocator {}

unsafe impl Sync for DlStdAllocator {}
