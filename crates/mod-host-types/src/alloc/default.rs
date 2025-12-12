use std::{
    mem::{self, ManuallyDrop},
    ptr::{self, NonNull},
};

use windows::Win32::System::Memory::{
    GetProcessHeap, HeapAlloc, HeapFree, HeapReAlloc, HeapSize, HEAP_NONE,
};

use super::{DlAllocator, DlAllocatorVtable, DlHeapDirection};

const PTR_SIZE: usize = mem::size_of::<*mut u8>();
const PTR_ALIGN: usize = mem::align_of::<*mut u8>();

pub const DEFAULT_DLALLOC: DlAllocator = DlAllocator {
    vtable: NonNull::from_ref(&DEFAULT_DLALLOC_VTABLE),
};

const DEFAULT_DLALLOC_VTABLE: DlAllocatorVtable = DlAllocatorVtable {
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

extern "C" fn dtor(_: NonNull<ManuallyDrop<DlAllocator>>) {}

extern "C" fn heap_id(_: NonNull<DlAllocator>) -> u32 {
    0x401
}

extern "C" fn allocator_id(_: NonNull<DlAllocator>) -> u32 {
    0xffffffff
}

extern "C" fn capability(
    _: NonNull<DlAllocator>,
    out: NonNull<u32>,
    _: DlHeapDirection,
) -> NonNull<u32> {
    unsafe {
        out.write(0x3b);
    }
    out
}

extern "C" fn total_size(_: NonNull<DlAllocator>) -> usize {
    usize::MAX
}

extern "C" fn free_size(_: NonNull<DlAllocator>) -> usize {
    usize::MAX
}

extern "C" fn max_size(_: NonNull<DlAllocator>) -> usize {
    usize::MAX
}

extern "C" fn num_blocks(_: NonNull<DlAllocator>) -> usize {
    0
}

extern "C" fn block_size(_: NonNull<DlAllocator>, block: *mut u8) -> usize {
    if let Some(block) = NonNull::new(block) {
        unsafe {
            if let Ok(heap) = GetProcessHeap() {
                let block_base = block_base(block);
                let block_size = HeapSize(heap, HEAP_NONE, block_base as _);

                return block_size - block.as_ptr().byte_offset_from(block_base) as usize;
            }
        }
    }
    0
}

extern "C" fn allocate(this: NonNull<DlAllocator>, size: usize) -> *mut u8 {
    allocate_aligned(this, size, 2 * PTR_ALIGN)
}

extern "C" fn allocate_aligned(_: NonNull<DlAllocator>, size: usize, alignment: usize) -> *mut u8 {
    if let Ok(heap) = unsafe { GetProcessHeap() } {
        if alignment == 0 || (alignment & (alignment - 1)) != 0 {
            return ptr::null_mut();
        }

        let alignment = alignment.max(2 * PTR_ALIGN);
        let align_val = alignment - 1;

        let Some(scratch) = size.checked_add(align_val + PTR_SIZE) else {
            return ptr::null_mut();
        };

        let block_base = unsafe { HeapAlloc(heap, HEAP_NONE, scratch) as *mut u8 };

        if !block_base.is_null() {
            let block = ((block_base as usize + align_val + PTR_SIZE) & !align_val) as *mut u8;

            unsafe {
                block.byte_sub(PTR_SIZE).cast::<*mut u8>().write(block_base);
            }

            return block;
        }
    }

    ptr::null_mut()
}

extern "C" fn reallocate(this: NonNull<DlAllocator>, old: *mut u8, new_size: usize) -> *mut u8 {
    reallocate_aligned(this, old, new_size, 2 * PTR_ALIGN)
}

extern "C" fn reallocate_aligned(
    this: NonNull<DlAllocator>,
    old: *mut u8,
    new_size: usize,
    alignment: usize,
) -> *mut u8 {
    let Some(old_block) = NonNull::new(old) else {
        return allocate(this, new_size);
    };

    if new_size == 0 {
        free(this, old);
        return ptr::null_mut();
    }

    unsafe {
        if let Ok(heap) = GetProcessHeap() {
            if alignment == 0 || (alignment & (alignment - 1)) != 0 {
                return ptr::null_mut();
            }

            let alignment = alignment.max(2 * PTR_ALIGN);
            let align_val = alignment - 1;

            let old_block_base = block_base(old_block);
            let old_block_size = HeapSize(heap, HEAP_NONE, old_block_base as _);

            let old_size = Ord::min(
                new_size,
                old_block_size - old_block.as_ptr().byte_offset_from(old_block_base) as usize,
            );

            let Some(scratch) = new_size.checked_add(align_val + PTR_SIZE) else {
                return ptr::null_mut();
            };

            let new_block_base =
                HeapReAlloc(heap, HEAP_NONE, Some(old_block_base as _), scratch) as *mut u8;

            if new_block_base == old_block_base && (new_block_base as usize & !align_val) == 0 {
                return new_block_base;
            }

            if !new_block_base.is_null() {
                let new_block =
                    ((new_block_base as usize + align_val + PTR_SIZE) & !align_val) as *mut u8;

                ptr::copy(old_block.as_ptr(), new_block, old_size);
                new_block
                    .byte_sub(PTR_SIZE)
                    .cast::<*mut u8>()
                    .write(new_block_base);

                return new_block;
            }
        }
    }

    ptr::null_mut()
}

extern "C" fn free(_: NonNull<DlAllocator>, ptr: *mut u8) {
    if let Some(ptr) = NonNull::new(ptr) {
        unsafe {
            if let Ok(heap) = GetProcessHeap() {
                let _ = HeapFree(heap, HEAP_NONE, Some(block_base(ptr) as _));
            }
        }
    }
}

extern "C" fn free_all(_: NonNull<DlAllocator>) {}

extern "C" fn back_allocate(this: NonNull<DlAllocator>, size: usize) -> *mut u8 {
    allocate(this, size)
}

extern "C" fn back_allocate_aligned(
    this: NonNull<DlAllocator>,
    size: usize,
    alignment: usize,
) -> *mut u8 {
    allocate_aligned(this, size, alignment)
}

extern "C" fn back_reallocate(
    this: NonNull<DlAllocator>,
    old: *mut u8,
    new_size: usize,
) -> *mut u8 {
    reallocate(this, old, new_size)
}

extern "C" fn back_reallocate_aligned(
    this: NonNull<DlAllocator>,
    old: *mut u8,
    new_size: usize,
    alignment: usize,
) -> *mut u8 {
    reallocate_aligned(this, old, new_size, alignment)
}

extern "C" fn back_free(this: NonNull<DlAllocator>, ptr: *mut u8) {
    free(this, ptr);
}

extern "C" fn self_diagnose(_: NonNull<DlAllocator>) -> bool {
    false
}

extern "C" fn is_valid_block(_: NonNull<DlAllocator>, _: *mut u8) -> bool {
    true
}

extern "C" fn lock(_: NonNull<DlAllocator>) {}

extern "C" fn unlock(_: NonNull<DlAllocator>) {}

extern "C" fn block_of(_: NonNull<DlAllocator>, ptr: *mut u8) -> *mut u8 {
    if let Some(ptr) = NonNull::new(ptr) {
        unsafe { block_base(ptr) }
    } else {
        ptr::null_mut()
    }
}

unsafe fn block_base(ptr: NonNull<u8>) -> *mut u8 {
    unsafe { ptr.byte_sub(PTR_SIZE).cast::<*mut u8>().read_unaligned() }
}
