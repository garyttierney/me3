//! `DLKR::DlAllocator` layout and compatible allocator for use in place of [`DlStdAllocator`]
//! from the `Dantelion2` in-house FromSoftware library.

use std::{
    alloc::{GlobalAlloc, Layout},
    mem::{self, ManuallyDrop},
    ptr::NonNull,
    sync::OnceLock,
};

use default::DEFAULT_DLALLOC;
use me3_binary_analysis::pe;
use pelite::pe::Pe;
use regex::bytes::Regex;
use thiserror::Error;

mod default;

#[derive(Clone, Debug, Error)]
pub enum AllocatorError {
    #[error(transparent)]
    Pe(#[from] pelite::Error),
    #[error("PE section \"{0}\" is missing")]
    Section(&'static str),
    #[error("pattern returned no matches")]
    Pattern,
    #[error("no allocator reported to own memory at {0:x}")]
    InvalidPtr(usize),
}

/// Commonly used polymorphic `DlAllocator` adapter for objects and containers.
///
/// Contains a pointer to a `DlAllocator` interface and implements [`GlobalAlloc`].
#[repr(transparent)]
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

impl DlStdAllocator {
    /// # Safety
    ///
    /// `object` must be individually heap allocated by one of the game's allocators.
    pub unsafe fn for_object<'a, P, T>(program: P, object: *const T) -> Result<Self, AllocatorError>
    where
        P: Pe<'a>,
    {
        static FOR_OBJECT: OnceLock<
            Result<extern "C" fn(*const u8) -> Option<DlStdAllocator>, AllocatorError>,
        > = OnceLock::new();

        let for_object = FOR_OBJECT.get_or_init(|| {
            let text_section = pe::section(program, ".text").map_err(AllocatorError::Section)?;
            let text = program.get_section_bytes(text_section)?;

            // matches:
            // movsxd r8,DWORD PTR [rip+??] (OR) mov r8d,DWORD PTR [rip+??]
            // xor    edx,edx (OPTIONAL)
            // test   r8,r8 (OR) test r8d,r8d
            // jle    ??
            // xor    edx,edx (OPTIONAL)
            // lea    rax,[rip+??]
            let re = Regex::new(
                r"(?s-u)(?:(?:\x44\x8b)|(?:\x4c\x63))\x05.{4}(?:\x33\xd2)?[\x45\x4d]\x85\xc0(?:(?:\x7e.)|(?:\x0f\x8e.{4}))(?:\x33\xd2)?\x48\x8d\x05.{4}",
            ).unwrap();

            let for_object_match = re.find(text)
                .ok_or(AllocatorError::Pattern)?.as_bytes().as_ptr();

            unsafe { Ok(mem::transmute(for_object_match)) }
        })
        .clone()?;

        for_object(object as _).ok_or(AllocatorError::InvalidPtr(object as _))
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
