use std::{mem, ops::Range, ptr::NonNull};

use me3_binary_analysis::pe;
use pelite::pe::Pe;
use regex::bytes::Regex;
use thiserror::Error;

type FileStepInit = unsafe extern "C" fn(usize);

pub fn find_init_fn<'a, P>(program: P) -> Result<FileStepInit, FindError>
where
    P: Pe<'a>,
{
    let [data, rdata] = pe::sections(program, [".data", ".rdata"]).map_err(FindError::Section)?;

    let data = program.get_section_bytes(data)?;
    let rdata = program.get_section_bytes(rdata)?;

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
    #[error(transparent)]
    Pe(#[from] pelite::Error),
    #[error("PE section \"{0}\" is missing")]
    Section(&'static str),
    #[error("step with name \"FileStep::STEP_Init\" not found")]
    Step,
    #[error("step method is null or not found")]
    Method,
}
