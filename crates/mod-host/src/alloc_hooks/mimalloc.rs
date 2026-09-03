use std::{
    env,
    ffi::c_void,
    fs, hint,
    mem::ManuallyDrop,
    os::windows::{ffi::OsStrExt, io::IntoRawHandle},
    ptr::NonNull,
    sync::{
        atomic::{AtomicU32, Ordering},
        LazyLock,
    },
};

use libmimalloc_sys::{
    mi_arena_id_t, mi_free, mi_heap_malloc_aligned, mi_heap_new_in_arena, mi_heap_realloc_aligned,
    mi_heap_t, mi_is_in_heap_region, mi_malloc_aligned, mi_manage_os_memory_ex, mi_option_set,
    mi_usable_size,
};
use me3_mod_host_types::{
    alloc::{DlAllocator, DlAllocatorVtable, DlHeapDirection},
    game::GAME,
};
use me3_mod_protocol::Game;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HANDLE, INVALID_HANDLE_VALUE},
        System::{
            Memory::{
                CreateFileMappingW, MapViewOfFile3, VirtualAlloc, MEM_COMMIT, MEM_RESERVE,
                PAGE_READWRITE, VIRTUAL_ALLOCATION_TYPE,
            },
            SystemServices::MEM_TOP_DOWN,
        },
    },
};

pub static MIMALLOC_DLALLOC: DlAllocator = DlAllocator {
    vtable: NonNull::from_ref(&MIMALLOC_DLALLOC_VTABLE),
};

static HEAP_SIZE_MB: AtomicU32 = AtomicU32::new(0);

/// Whether `ptr` can safely be passed to a mimalloc operation that reads block metadata.
///
/// The game may retain allocations made before its allocator table is patched. More importantly,
/// a corrupt or sentinel pointer must not reach mimalloc: mimalloc derives its page metadata from
/// the pointer, so even a value such as `0x118` can turn into an access violation near null.
fn is_mimalloc_block(ptr: *mut u8) -> bool {
    !ptr.is_null() && unsafe { mi_is_in_heap_region(ptr.cast()) }
}

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
    let mut size_mb = HEAP_SIZE_MB.load(Ordering::Acquire) as usize;
    if size_mb == 0 {
        size_mb = match *GAME {
            Game::DarkSouls3 => 6 * 1024,
            Game::Sekiro => 6 * 1024,
            Game::EldenRing => 12 * 1024,
            _ => unimplemented!("this game does not support mem_patch?"),
        };
    }

    let size = size_mb * 1024 * 1024;

    let ptr = alloc_os_memory(size).expect("failed to allocate OS memory");
    let mut arena_id = mi_arena_id_t::default();

    if !mi_manage_os_memory_ex(ptr, size, true, false, false, -1, true, &mut arena_id) {
        panic!("mimalloc failed to manage OS memory");
    }

    mi_heap_new_in_arena(arena_id)
});

unsafe fn alloc_os_memory(size: usize) -> Result<*mut c_void, eyre::Error> {
    // Dev env vars to allow heap memory introspection from other processes.
    // Path to a backing (on-disk) file for the heap memory.
    let mapping_file = env::var_os("ME3_HEAP_MAPPING_FILE");
    // Kernel object name to use with CreateFileMapping.
    let mapping_name = env::var_os("ME3_HEAP_MAPPING_NAME");

    let ptr = if mapping_file.is_none() && mapping_name.is_none() {
        // Neither var is set, don't bother with a memory mapping.
        unsafe {
            VirtualAlloc(
                None,
                size,
                MEM_COMMIT | MEM_RESERVE | VIRTUAL_ALLOCATION_TYPE(MEM_TOP_DOWN),
                PAGE_READWRITE,
            )
        }
    } else {
        let handle = match mapping_file {
            Some(path) => {
                let file = fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(path)?;

                HANDLE(file.into_raw_handle())
            }
            None => INVALID_HANDLE_VALUE,
        };

        let mut name = vec![];
        let name = match mapping_name {
            Some(var) => {
                name = var.encode_wide().chain([0]).collect();
                PCWSTR::from_raw(name.as_ptr())
            }
            None => PCWSTR::null(),
        };

        let mapping = unsafe {
            CreateFileMappingW(
                handle,
                None,
                PAGE_READWRITE,
                (size as u64 >> 32) as u32,
                size as u32,
                name,
            )?
        };

        // N.B. MEM_TOP_DOWN support is undocumented but works on Windows and Wine.
        unsafe {
            MapViewOfFile3(
                mapping,
                None,
                None,
                0,
                0,
                VIRTUAL_ALLOCATION_TYPE(MEM_TOP_DOWN),
                PAGE_READWRITE.0,
                None,
            )
            .Value
        }
    };

    if ptr.is_null() {
        return Err(windows::core::Error::from_thread().into());
    }

    Ok(ptr)
}

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
    if is_mimalloc_block(block) {
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
    let size = size.next_multiple_of(alignment);
    let mut ptr = unsafe { mi_heap_malloc_aligned(*MI_HEAP, size, alignment) };
    if ptr.is_null() {
        hint::cold_path();
        ptr = unsafe { mi_malloc_aligned(size, alignment) };
    }
    ptr as _
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
    if !old.is_null() && !is_mimalloc_block(old) {
        return std::ptr::null_mut();
    }
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
    if is_mimalloc_block(ptr) {
        unsafe {
            mi_free(ptr as _);
        }
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

unsafe extern "C" fn is_valid_block(_: NonNull<DlAllocator>, block: *mut u8) -> bool {
    is_mimalloc_block(block)
}

unsafe extern "C" fn lock(_: NonNull<DlAllocator>) {}

unsafe extern "C" fn unlock(_: NonNull<DlAllocator>) {}

unsafe extern "C" fn block_of(_: NonNull<DlAllocator>, _: *mut u8) -> *mut u8 {
    std::ptr::null_mut()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teardown_near_null_pointer_is_rejected_before_mimalloc_metadata_access() {
        let pointer = 0x118usize as *mut u8;
        let allocator = NonNull::from_ref(&MIMALLOC_DLALLOC);

        assert!(!is_mimalloc_block(pointer));
        assert_eq!(unsafe { block_size(allocator, pointer) }, 0);
        assert!(!unsafe { is_valid_block(allocator, pointer) });
        assert!(unsafe { reallocate_aligned(allocator, pointer, 64, 16) }.is_null());
        assert!(unsafe { back_reallocate_aligned(allocator, pointer, 64, 16) }.is_null());
        unsafe { free(allocator, pointer) };
        unsafe { back_free(allocator, pointer) };
    }

    #[test]
    fn null_pointer_remains_a_free_noop_and_a_realloc_allocation_request() {
        assert!(!is_mimalloc_block(std::ptr::null_mut()));
        unsafe { free(NonNull::from_ref(&MIMALLOC_DLALLOC), std::ptr::null_mut()) };
    }

    #[test]
    fn mimalloc_owned_block_still_reaches_size_validity_and_free_paths() {
        let pointer = unsafe { mi_malloc_aligned(64, 16) }.cast::<u8>();
        assert!(!pointer.is_null());

        let allocator = NonNull::from_ref(&MIMALLOC_DLALLOC);
        assert!(is_mimalloc_block(pointer));
        assert!(unsafe { is_valid_block(allocator, pointer) });
        assert!(unsafe { block_size(allocator, pointer) } >= 64);
        unsafe { free(allocator, pointer) };
    }
}
