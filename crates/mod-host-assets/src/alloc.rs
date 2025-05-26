//! `DLKR::DLAllocator` layout and compatible allocator for use in place of [`DLStdAllocator`]
//! from the `Dantelion2` in-house FromSoftware library.

use std::{
    alloc::{GlobalAlloc, Layout},
    mem::ManuallyDrop,
    ptr::NonNull,
};

/// Commonly used polymorphic `DLAllocator` adapter for objects and containers.
///
/// Contains a pointer to a `DLAllocator` interface and implements [`GlobalAlloc`].
#[repr(C)]
#[derive(Clone, Debug)]
pub struct DLStdAllocator {
    inner: NonNull<DLAllocator>,
}

#[repr(C)]
struct DLAllocator {
    vtable: NonNull<DLAllocatorVtable>,
}

#[repr(u32)]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default)]
enum DLHeapDirection {
    #[default]
    Front = 0,
    Back = 1,
}

#[repr(C)]
struct DLAllocatorVtable {
    dtor: extern "C" fn(this: NonNull<ManuallyDrop<DLAllocator>>),

    heap_id: extern "C" fn(this: NonNull<DLAllocator>) -> u32,

    allocator_id: extern "C" fn(this: NonNull<DLAllocator>) -> u32,

    capability: extern "C" fn(
        this: NonNull<DLAllocator>,
        out: NonNull<u32>,
        heap: DLHeapDirection,
    ) -> NonNull<u32>,

    total_size: extern "C" fn(this: NonNull<DLAllocator>) -> usize,

    free_size: extern "C" fn(this: NonNull<DLAllocator>) -> usize,

    max_size: extern "C" fn(this: NonNull<DLAllocator>) -> usize,

    num_blocks: extern "C" fn(this: NonNull<DLAllocator>) -> usize,

    block_size: extern "C" fn(this: NonNull<DLAllocator>, block: *mut u8) -> usize,

    allocate: extern "C" fn(this: NonNull<DLAllocator>, size: usize) -> *mut u8,

    allocate_aligned:
        extern "C" fn(this: NonNull<DLAllocator>, size: usize, alignment: usize) -> *mut u8,

    reallocate: extern "C" fn(this: NonNull<DLAllocator>, old: *mut u8, new_size: usize) -> *mut u8,

    reallocate_aligned: extern "C" fn(
        this: NonNull<DLAllocator>,
        old: *mut u8,
        new_size: usize,
        alignment: usize,
    ) -> *mut u8,

    free: extern "C" fn(this: NonNull<DLAllocator>, ptr: *mut u8),

    free_all: extern "C" fn(this: NonNull<DLAllocator>),

    back_allocate: extern "C" fn(this: NonNull<DLAllocator>, size: usize) -> *mut u8,

    back_allocate_aligned:
        extern "C" fn(this: NonNull<DLAllocator>, size: usize, alignment: usize) -> *mut u8,

    back_reallocate:
        extern "C" fn(this: NonNull<DLAllocator>, old: *mut u8, new_size: usize) -> *mut u8,

    back_reallocate_aligned: extern "C" fn(
        this: NonNull<DLAllocator>,
        old: *mut u8,
        new_size: usize,
        alignment: usize,
    ) -> *mut u8,

    back_free: extern "C" fn(this: NonNull<DLAllocator>, ptr: *mut u8),

    self_diagnose: extern "C" fn(this: NonNull<DLAllocator>) -> bool,

    is_valid_block: extern "C" fn(this: NonNull<DLAllocator>, block: *mut u8) -> bool,

    lock: extern "C" fn(this: NonNull<DLAllocator>),

    unlock: extern "C" fn(this: NonNull<DLAllocator>),

    block_of: extern "C" fn(this: NonNull<DLAllocator>, ptr: *mut u8) -> *mut u8,
}

unsafe impl GlobalAlloc for DLStdAllocator {
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

unsafe impl Send for DLAllocator {}

unsafe impl Sync for DLAllocator {}

unsafe impl Send for DLStdAllocator {}

unsafe impl Sync for DLStdAllocator {}
