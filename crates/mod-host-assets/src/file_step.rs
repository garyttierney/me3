use std::mem;

use me3_binary_analysis::pe;
use pelite::pe::Pe;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
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

    // "FileStep" preceded by up to 15 other characters, as a UTF-16 string.
    // Used to find "SPRJFileStep" or "CSFileStep".
    let step_name_re = Regex::new(
        r"(?s-u)(?:\w\x00){0,15}F\x00i\x00l\x00e\x00S\x00t\x00e\x00p\x00:\x00:\x00S\x00T\x00E\x00P\x00_\x00I\x00n\x00i\x00t\x00\x00\x00",
    )
    .unwrap();

    let strings = step_name_re
        .find_iter(rdata)
        .map(|m| m.as_bytes().as_ptr() as usize)
        .collect::<Vec<_>>();

    if strings.is_empty() {
        return Err(FindError::Step);
    }

    let (_, data_ptrs, _) = unsafe { data.align_to::<usize>() };

    let step_name_ptr = &raw const *data_ptrs
        .par_iter()
        .find_any(|ptr| strings.contains(*ptr))
        .ok_or(FindError::Step)?;

    unsafe {
        let fn_ptr = step_name_ptr.wrapping_sub(1).read();

        if fn_ptr != 0 {
            Ok(mem::transmute(fn_ptr))
        } else {
            Err(FindError::Method)
        }
    }
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
