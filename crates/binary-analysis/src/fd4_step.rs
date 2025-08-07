use std::{
    ffi::OsString, marker::PhantomData, ops::Range, os::windows::ffi::OsStringExt, ptr::NonNull,
};

use pelite::{
    pe::{Pe, Va},
    Align,
};
use rayon::{
    iter::ParallelIterator,
    slice::{ParallelSlice, ParallelSliceMut},
};
use regex::bytes::Regex;
use thiserror::Error;

use crate::pe::sections;

pub type Fd4StepFunction = unsafe extern "C" fn(this: NonNull<()>);

#[derive(Debug, Error)]
pub enum Fd4StepError {
    #[error(transparent)]
    Pelite(#[from] pelite::Error),
    #[error("PE section \"{0}\" is missing")]
    Section(&'static str),
}

#[derive(Clone, Debug)]
pub struct Fd4StepTables<'a> {
    inner: Box<[(Box<str>, *mut Option<Fd4StepFunction>)]>,
    _marker: PhantomData<&'a Fd4StepFunction>,
}

impl<'a> Fd4StepTables<'a> {
    pub fn by_name<S: AsRef<str>>(&self, name: S) -> Option<Fd4StepFunction> {
        match self
            .inner
            .binary_search_by_key(&name.as_ref(), |(name, _)| name.as_ref())
        {
            Ok(pos) => unsafe { self.inner[pos].1.read() },
            Err(_) => None,
        }
    }

    /// Find FD4 step functions in the assembly of the static initializers
    /// that construct the step tables.
    ///
    /// Will not find all entries in the case of obfuscated or encrypted code,
    /// but can be applied statically as opposed to [`Fd4StepTables::from_initialized_data`].
    pub fn from_static_initializers<P>(program: P) -> Result<Self, Fd4StepError>
    where
        P: Pe<'a>,
    {
        let [text, data, rdata] =
            sections(program, [".text", ".data", ".rdata"]).map_err(Fd4StepError::Section)?;

        let image_base = program.image().as_ptr();

        let text_virtual_range = text.virtual_range();
        let data_virtual_range = data.virtual_range();
        let rdata_virtual_range = rdata.virtual_range();

        // Matches:
        // lea    rax,[rip+??]
        // mov    QWORD PTR [rip+??],rax
        // lea    rax,[rip+??]
        // mov    QWORD PTR [rip+??],rax
        let re = Regex::new(
            r"(?s-u)\x48\x8d\x05(.{4})\x48\x89\x05(.{4})\x48\x8d\x05(.{4})\x48\x89\x05(.{4})",
        )
        .unwrap();

        let mut step_fns = re
            .captures_iter(program.get_section_bytes(text)?)
            .filter_map(|c| {
                let [fn_src, fn_dst, name_src, name_dst] = if program.align() == Align::File {
                    let [Ok(fn_src), Ok(fn_dst), Ok(name_src), Ok(name_dst)] =
                        c.extract().1.map(|disp32| unsafe {
                            program
                                .file_offset_to_rva(
                                    disp32.as_ptr_range().end.offset_from_unsigned(image_base),
                                )
                                .map(|rva| {
                                    let disp32 = i32::from_le_bytes(disp32.try_into().unwrap());
                                    rva.wrapping_add(disp32 as u32)
                                })
                        })
                    else {
                        return None;
                    };

                    [fn_src, fn_dst, name_src, name_dst]
                } else {
                    c.extract().1.map(|disp32| unsafe {
                        let rva = disp32.as_ptr_range().end.offset_from_unsigned(image_base) as u32;
                        let disp32 = i32::from_le_bytes(disp32.try_into().unwrap());
                        rva.wrapping_add(disp32 as u32)
                    })
                };

                if !fn_src.is_multiple_of(16)
                    || !name_src.is_multiple_of(8)
                    || !fn_dst.is_multiple_of(8)
                    || !fn_dst.is_multiple_of(8)
                {
                    return None;
                }

                if !text_virtual_range.contains(&fn_src)
                    || !rdata_virtual_range.contains(&name_src)
                    || !data_virtual_range.contains(&fn_dst)
                    || !data_virtual_range.contains(&name_dst)
                {
                    return None;
                }

                let name = program.derva_slice_s(name_src, 0u16).ok()?;

                if !name.iter().all(|c| {
                    matches!(
                        char::from_u32(*c as u32), Some(c) if c.is_ascii_alphanumeric()
                            || c == '_'
                            || c == ':'
                    )
                }) {
                    return None;
                }

                let name = OsString::from_wide(name).to_string_lossy().into_owned();

                Some((name, program.derva::<Va>(fn_src).ok()?))
            })
            .collect::<Vec<_>>();

        step_fns.par_sort_by(|(a, _), (b, _)| a.cmp(b));
        step_fns.dedup_by(|(a, _), (b, _)| a == b);

        Ok(Self {
            inner: step_fns_into_inner(step_fns),
            _marker: PhantomData,
        })
    }

    /// Find FD4 step functions in the initialized tables in the .data section.
    ///
    /// The static initializers must have ran for this to return any matches,
    /// therefore this function is only sensible to call at runtime.
    ///
    /// However, function-level obfuscation and encryption is not a problem
    /// as opposed to [`Fd4StepTables::from_static_initializers`].
    pub fn from_initialized_data<P>(program: P) -> Result<Self, Fd4StepError>
    where
        P: Pe<'a> + Send + Sync,
    {
        let [data, rdata] =
            sections(program, [".data", ".rdata"]).map_err(Fd4StepError::Section)?;

        let data_bytes = program.get_section_bytes(data)?;

        let rdata_virtual_range = rdata.virtual_range();

        let rdata_range = Range {
            start: program.rva_to_va(rdata_virtual_range.start)?,
            end: program.rva_to_va(rdata_virtual_range.end)?,
        };

        let (_, data_ptrs, _) = unsafe { data_bytes.align_to::<Va>() };

        let mut step_fns = data_ptrs
            .par_windows(2)
            .filter_map(|w| {
                let (fn_dst, name_src) = (&w[0], w[1]);

                if !name_src.is_multiple_of(8) || !rdata_range.contains(&name_src) {
                    return None;
                }

                let name = program
                    .va_to_rva(name_src)
                    .and_then(|rva| program.derva_slice_s(rva, 0u16))
                    .ok()?;

                if !name.iter().all(|c| {
                    matches!(
                        char::from_u32(*c as u32), Some(c) if c.is_ascii_alphanumeric()
                            || c == '_'
                            || c == ':'
                    )
                }) {
                    return None;
                }

                let name = OsString::from_wide(name).to_string_lossy().into_owned();

                Some((name, fn_dst))
            })
            .collect::<Vec<_>>();

        step_fns.par_sort_by(|(a, _), (b, _)| a.cmp(b));
        step_fns.dedup_by(|(a, _), (b, _)| a == b);

        Ok(Self {
            inner: step_fns_into_inner(step_fns),
            _marker: PhantomData,
        })
    }
}

fn step_fns_into_inner(
    step_fns: Vec<(String, &u64)>,
) -> Box<[(Box<str>, *mut Option<Fd4StepFunction>)]> {
    Vec::into_boxed_slice(
        step_fns
            .into_iter()
            .map(|(name, ptr)| {
                (
                    name.into_boxed_str(),
                    &raw const *ptr as *mut Option<Fd4StepFunction>,
                )
            })
            .collect(),
    )
}
