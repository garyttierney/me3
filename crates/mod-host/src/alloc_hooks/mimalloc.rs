use std::{
    mem::ManuallyDrop,
    ptr::NonNull,
    sync::{
        atomic::{AtomicU32, Ordering},
        LazyLock,
    },
};

use libmimalloc_sys::{
    mi_arena_id_t, mi_free, mi_heap_malloc_aligned, mi_heap_new_in_arena, mi_heap_realloc_aligned,
    mi_heap_t, mi_option_set, mi_reserve_os_memory_ex, mi_usable_size,
};
use me3_mod_host_types::{
    alloc::{DlAllocator, DlAllocatorVtable, DlHeapDirection},
    game::GAME,
};
use me3_mod_protocol::Game;

pub static MIMALLOC_DLALLOC: DlAllocator = DlAllocator {
    vtable: NonNull::from_ref(&MIMALLOC_DLALLOC_VTABLE),
};

static HEAP_SIZE_MB: AtomicU32 = AtomicU32::new(0);

pub fn set_heap_size(new_size_mb: u32) {
    HEAP_SIZE_MB.store(new_size_mb, Ordering::Release);
}

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

/// A contiguous, pre-committed heap used in place of mimalloc's default heap to reduce
/// crashes caused by bad memory accesses.
static mut MI_HEAP: LazyLock<*mut mi_heap_t> = LazyLock::new(|| unsafe {
    // Disable decommitting purged pages to reduce the likelihood of crashing
    // because of use-after-free and OOB access bugs. This is especially important in Sekiro,
    // which consistently causes OOB reads in a certain `GXFlverMaterial` function (these reads
    // may cross page boundaries and segfault on a decommitted page when this option is "1").
    let mi_option_purge_decommits = 5;
    mi_option_set(mi_option_purge_decommits, 0);

    // Assuming an overcommit system, numbers are based on how much memory the games already
    // commit, manual testing and expectations for upper bounds on memory usage.
    // Since `mi_option_disallow_os_alloc` is not set mimalloc may reserve even more memory
    // outside of this arena as it needs.
    let mut size_mb = HEAP_SIZE_MB.load(Ordering::Acquire);
    if size_mb == 0 {
        size_mb = match *GAME {
            Game::DarkSouls3 => 6 * 1024,
            Game::Sekiro => 6 * 1024,
            Game::EldenRing => 12 * 1024,
            _ => unimplemented!("this game does not support mem_patch?"),
        };
    }

    let size_bytes = size_mb as usize * 1024 * 1024;
    let mut arena_id = mi_arena_id_t::default();

    let res = mi_reserve_os_memory_ex(size_bytes, true, true, true, &mut arena_id);

    if res != 0 {
        panic!("mimalloc failed to reserve and commit OS memory");
    }

    mi_heap_new_in_arena(arena_id)
});

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

unsafe extern "C" fn allocate(this: NonNull<DlAllocator>, size: usize) -> *mut u8 {
    unsafe { allocate_aligned(this, size, 16) }
}

unsafe extern "C" fn allocate_aligned(
    _: NonNull<DlAllocator>,
    size: usize,
    alignment: usize,
) -> *mut u8 {
    let alignment = alignment.max(16);
    unsafe { mi_heap_malloc_aligned(*MI_HEAP, size.next_multiple_of(alignment), alignment) as _ }
}

unsafe extern "C" fn reallocate(
    this: NonNull<DlAllocator>,
    old: *mut u8,
    new_size: usize,
) -> *mut u8 {
    unsafe { reallocate_aligned(this, old, new_size, 16) }
}

unsafe extern "C" fn reallocate_aligned(
    _: NonNull<DlAllocator>,
    old: *mut u8,
    new_size: usize,
    alignment: usize,
) -> *mut u8 {
    let alignment = alignment.max(16);
    unsafe {
        mi_heap_realloc_aligned(
            *MI_HEAP,
            old as _,
            new_size.next_multiple_of(alignment),
            alignment,
        ) as _
    }
}

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
