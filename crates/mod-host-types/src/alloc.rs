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
pub struct DlAllocator {
    pub vtable: NonNull<DlAllocatorVtable>,
}

#[repr(u32)]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default)]
pub enum DlHeapDirection {
    #[default]
    Front = 0,
    Back = 1,
}

#[repr(C)]
pub struct DlAllocatorVtable {
    pub dtor: unsafe extern "C" fn(this: NonNull<ManuallyDrop<DlAllocator>>),

    pub heap_id: unsafe extern "C" fn(this: NonNull<DlAllocator>) -> u32,

    pub allocator_id: unsafe extern "C" fn(this: NonNull<DlAllocator>) -> u32,

    pub capability: unsafe extern "C" fn(
        this: NonNull<DlAllocator>,
        out: NonNull<u32>,
        heap: DlHeapDirection,
    ) -> NonNull<u32>,

    pub total_size: unsafe extern "C" fn(this: NonNull<DlAllocator>) -> usize,

    pub free_size: unsafe extern "C" fn(this: NonNull<DlAllocator>) -> usize,

    pub max_size: unsafe extern "C" fn(this: NonNull<DlAllocator>) -> usize,

    pub num_blocks: unsafe extern "C" fn(this: NonNull<DlAllocator>) -> usize,

    pub block_size: unsafe extern "C" fn(this: NonNull<DlAllocator>, block: *mut u8) -> usize,

    pub allocate: unsafe extern "C" fn(this: NonNull<DlAllocator>, size: usize) -> *mut u8,

    pub allocate_aligned:
        unsafe extern "C" fn(this: NonNull<DlAllocator>, size: usize, alignment: usize) -> *mut u8,

    pub reallocate:
        unsafe extern "C" fn(this: NonNull<DlAllocator>, old: *mut u8, new_size: usize) -> *mut u8,

    pub reallocate_aligned: unsafe extern "C" fn(
        this: NonNull<DlAllocator>,
        old: *mut u8,
        new_size: usize,
        alignment: usize,
    ) -> *mut u8,

    pub free: unsafe extern "C" fn(this: NonNull<DlAllocator>, ptr: *mut u8),

    pub free_all: unsafe extern "C" fn(this: NonNull<DlAllocator>),

    pub back_allocate: unsafe extern "C" fn(this: NonNull<DlAllocator>, size: usize) -> *mut u8,

    pub back_allocate_aligned:
        unsafe extern "C" fn(this: NonNull<DlAllocator>, size: usize, alignment: usize) -> *mut u8,

    pub back_reallocate:
        unsafe extern "C" fn(this: NonNull<DlAllocator>, old: *mut u8, new_size: usize) -> *mut u8,

    pub back_reallocate_aligned: unsafe extern "C" fn(
        this: NonNull<DlAllocator>,
        old: *mut u8,
        new_size: usize,
        alignment: usize,
    ) -> *mut u8,

    pub back_free: unsafe extern "C" fn(this: NonNull<DlAllocator>, ptr: *mut u8),

    pub self_diagnose: unsafe extern "C" fn(this: NonNull<DlAllocator>) -> bool,

    pub is_valid_block: unsafe extern "C" fn(this: NonNull<DlAllocator>, block: *mut u8) -> bool,

    pub lock: unsafe extern "C" fn(this: NonNull<DlAllocator>),

    pub unlock: unsafe extern "C" fn(this: NonNull<DlAllocator>),

    pub block_of: unsafe extern "C" fn(this: NonNull<DlAllocator>, ptr: *mut u8) -> *mut u8,
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

    pub fn into_inner(self) -> NonNull<DlAllocator> {
        self.inner
    }
}

unsafe impl GlobalAlloc for DlStdAllocator {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let alignment = layout.align();

        let vtable = unsafe { self.inner.as_ref().vtable.as_ref() };
        unsafe { (vtable.allocate_aligned)(self.inner, size, alignment) }
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) {
        let vtable = unsafe { self.inner.as_ref().vtable.as_ref() };
        unsafe {
            (vtable.free)(self.inner, ptr);
        }
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
