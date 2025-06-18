use std::{mem, ops::Range, ptr::NonNull};

use regex::bytes::Regex;
use thiserror::Error;

use crate::pe;

type FileStepInit = unsafe extern "C" fn(usize);

/// # Safety
/// [`pelite::pe64::PeView::module`] must be safe to call on `image_base`
pub unsafe fn find_init_fn(image_base: *const u8) -> Result<FileStepInit, FindError> {
    // SAFETY: must be upheld by caller.
    let [data, rdata] = unsafe { pe::sections(image_base, [".data", ".rdata"])? };

    let step_name_re = Regex::new(
        r"(?s-u)(?:\w\x00){0,15}F\x00i\x00l\x00e\x00S\x00t\x00e\x00p\x00:\x00:\x00S\x00T\x00E\x00P\x00_\x00I\x00n\x00i\x00t\x00\x00\x00",
    )
    .unwrap();

    let strings = step_name_re
        .find_iter(rdata)
        .map(|m| m.as_bytes().as_ptr())
        .collect::<Vec<_>>();

    if strings.is_empty() {
        return Err(FindError::Step);
    }

    const SIZE: usize = mem::size_of::<*const u8>();
    const ALIGNMENT: usize = mem::align_of::<*const u8>();

    let Range { start, end } = data.as_ptr_range();

    let mut data_ptr = start.wrapping_byte_offset(start.align_offset(ALIGNMENT) as isize);

    let data_end = end.wrapping_byte_sub(SIZE);

    while data_ptr < data_end {
        // SAFETY: pointer is aligned and non-null.
        let fn_ptr = unsafe { data_ptr.cast::<*mut u8>().read() };

        data_ptr = data_ptr.wrapping_byte_add(SIZE);

        // SAFETY: pointer is aligned and non-null.
        let name_ptr = unsafe { data_ptr.cast::<*const u8>().read() };

        if strings.contains(&name_ptr) {
            let fn_ptr = NonNull::new(fn_ptr).ok_or(FindError::Method)?;

            // SAFETY: non-null function pointer conversion.
            return unsafe { Ok(mem::transmute(fn_ptr.as_ptr())) };
        }
    }

    Err(FindError::Method)
}

#[derive(Error, Debug)]
pub enum FindError {
    #[error("{0}")]
    PeSection(pe::SectionError),
    #[error("step with name \"FileStep::STEP_Init\" not found")]
    Step,
    #[error("step method table not found")]
    Table,
    #[error("step method is null or not found")]
    Method,
}

impl From<pe::SectionError> for FindError {
    fn from(value: pe::SectionError) -> Self {
        FindError::PeSection(value)
    }
}
