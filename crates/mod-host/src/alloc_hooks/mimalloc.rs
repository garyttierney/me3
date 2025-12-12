use std::{mem::ManuallyDrop, ptr::NonNull};

use libmimalloc_sys::{mi_free, mi_malloc_aligned, mi_realloc_aligned, mi_usable_size};
use me3_mod_host_types::alloc::{DlAllocator, DlAllocatorVtable, DlHeapDirection};

pub static MIMALLOC_DLALLOC: DlAllocator = DlAllocator {
    vtable: NonNull::from_ref(&MIMALLOC_DLALLOC_VTABLE),
};

const MIMALLOC_DLALLOC_VTABLE: DlAllocatorVtable = DlAllocatorVtable {
    dtor,
    heap_id,
    allocator_id,
    capability,
    total_size,
    free_size,
    max_size,
    num_blocks,
    block_size,
    allocate,
    allocate_aligned,
    reallocate,
    reallocate_aligned,
    free,
    free_all,
    back_allocate,
    back_allocate_aligned,
    back_reallocate,
    back_reallocate_aligned,
    back_free,
    self_diagnose,
    is_valid_block,
    lock,
    unlock,
    block_of,
};

unsafe extern "C" fn dtor(_: NonNull<ManuallyDrop<DlAllocator>>) {}

unsafe extern "C" fn heap_id(_: NonNull<DlAllocator>) -> u32 {
    0x401
}

unsafe extern "C" fn allocator_id(_: NonNull<DlAllocator>) -> u32 {
    0xffffffff
}

unsafe extern "C" fn capability(
    _: NonNull<DlAllocator>,
    out: NonNull<u32>,
    _: DlHeapDirection,
) -> NonNull<u32> {
    unsafe {
        out.write(0x7b);
    }
    out
}

unsafe extern "C" fn total_size(_: NonNull<DlAllocator>) -> usize {
    usize::MAX
}

unsafe extern "C" fn free_size(_: NonNull<DlAllocator>) -> usize {
    usize::MAX
}

unsafe extern "C" fn max_size(_: NonNull<DlAllocator>) -> usize {
    usize::MAX
}

unsafe extern "C" fn num_blocks(_: NonNull<DlAllocator>) -> usize {
    0
}

unsafe extern "C" fn block_size(_: NonNull<DlAllocator>, block: *mut u8) -> usize {
    if !block.is_null() {
        unsafe { mi_usable_size(block as _) }
    } else {
        0
    }
}

#[inline]
unsafe extern "C" fn allocate(_: NonNull<DlAllocator>, size: usize) -> *mut u8 {
    unsafe { mi_malloc_aligned(size.next_multiple_of(16), 16) as _ }
}

#[inline]
unsafe extern "C" fn allocate_aligned(
    _: NonNull<DlAllocator>,
    size: usize,
    alignment: usize,
) -> *mut u8 {
    let alignment = alignment.max(16);
    unsafe { mi_malloc_aligned(size.next_multiple_of(alignment), alignment) as _ }
}

#[inline]
unsafe extern "C" fn reallocate(_: NonNull<DlAllocator>, old: *mut u8, new_size: usize) -> *mut u8 {
    unsafe { mi_realloc_aligned(old as _, new_size.next_multiple_of(16), 16) as _ }
}

#[inline]
unsafe extern "C" fn reallocate_aligned(
    _: NonNull<DlAllocator>,
    old: *mut u8,
    new_size: usize,
    alignment: usize,
) -> *mut u8 {
    let alignment = alignment.max(16);
    unsafe { mi_realloc_aligned(old as _, new_size.next_multiple_of(alignment), alignment) as _ }
}

#[inline]
unsafe extern "C" fn free(_: NonNull<DlAllocator>, ptr: *mut u8) {
    unsafe {
        mi_free(ptr as _);
    }
}

unsafe extern "C" fn free_all(_: NonNull<DlAllocator>) {}

unsafe extern "C" fn back_allocate(this: NonNull<DlAllocator>, size: usize) -> *mut u8 {
    unsafe { allocate(this, size) }
}

unsafe extern "C" fn back_allocate_aligned(
    this: NonNull<DlAllocator>,
    size: usize,
    alignment: usize,
) -> *mut u8 {
    unsafe { allocate_aligned(this, size, alignment) }
}

unsafe extern "C" fn back_reallocate(
    this: NonNull<DlAllocator>,
    old: *mut u8,
    new_size: usize,
) -> *mut u8 {
    unsafe { reallocate(this, old, new_size) }
}

unsafe extern "C" fn back_reallocate_aligned(
    this: NonNull<DlAllocator>,
    old: *mut u8,
    new_size: usize,
    alignment: usize,
) -> *mut u8 {
    unsafe { reallocate_aligned(this, old, new_size, alignment) }
}

unsafe extern "C" fn back_free(this: NonNull<DlAllocator>, ptr: *mut u8) {
    unsafe {
        free(this, ptr);
    }
}

unsafe extern "C" fn self_diagnose(_: NonNull<DlAllocator>) -> bool {
    false
}

unsafe extern "C" fn is_valid_block(_: NonNull<DlAllocator>, _: *mut u8) -> bool {
    true
}

unsafe extern "C" fn lock(_: NonNull<DlAllocator>) {}

unsafe extern "C" fn unlock(_: NonNull<DlAllocator>) {}

unsafe extern "C" fn block_of(_: NonNull<DlAllocator>, _: *mut u8) -> *mut u8 {
    std::ptr::null_mut()
}
