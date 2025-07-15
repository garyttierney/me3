use std::{collections::HashMap, ffi::CStr, ops::Range, ptr};

use pelite::pe::{
    msvc::{
        RTTIBaseClassDescriptor, RTTIClassHierarchyDescriptor, RTTICompleteObjectLocator,
        TypeDescriptor, PMD,
    },
    Pe, Rva, Va,
};
use rayon::{
    iter::{IntoParallelIterator, ParallelIterator},
    slice::ParallelSlice,
};
use thiserror::Error;

use crate::pe::sections;

#[derive(Error, Debug)]
pub enum RttiError {
    #[error(transparent)]
    Pelite(#[from] pelite::Error),
    #[error("PE section \"{0}\" is missing")]
    Section(&'static str),
    #[error("index out of bounds")]
    Bounds,
}

pub type ClassMap = HashMap<Box<str>, Box<[UntypedVmt]>>;

#[derive(Clone, Copy, Debug)]
pub struct UntypedVmt(*const Va);

#[derive(Clone)]
pub struct ClassCol<'a, P>
where
    P: Pe<'a>,
{
    program: P,
    inner: &'a RTTICompleteObjectLocator,
}

#[derive(Clone)]
pub struct BaseClasses<'a, P>
where
    P: Pe<'a>,
{
    program: P,
    attributes: u32,
    inner: &'a [Rva],
}

#[derive(Clone)]
pub struct BaseClass<'a, P>
where
    P: Pe<'a>,
{
    program: P,
    inner: &'a RTTIBaseClassDescriptor,
}

pub fn classes<'a, P>(program: P) -> Result<ClassMap, RttiError>
where
    P: Pe<'a> + Send + Sync,
{
    let [text, data, rdata] =
        sections(program, [".text", ".data", ".rdata"]).map_err(RttiError::Section)?;

    let rdata_bytes = program.get_section_bytes(rdata)?;

    let text_virtual_range = text.virtual_range();
    let data_virtual_range = data.virtual_range();
    let rdata_virtual_range = rdata.virtual_range();

    let text_range = Range {
        start: program.rva_to_va(text_virtual_range.start)?,
        end: program.rva_to_va(text_virtual_range.end)?,
    };

    let rdata_range = Range {
        start: program.rva_to_va(rdata_virtual_range.start)?,
        end: program.rva_to_va(rdata_virtual_range.end)?,
    };

    let (_, rdata_ptrs, _) = unsafe { rdata_bytes.align_to::<Va>() };

    let mut possible_vtables = rdata_ptrs
        .par_windows(2)
        .filter_map(|w| {
            if let [col, pfn] = w
                && rdata_range.contains(col)
                && text_range.contains(pfn)
            {
                let col: &RTTICompleteObjectLocator = program
                    .va_to_rva(*col)
                    .and_then(|rva| program.derva(rva))
                    .ok()?;

                if data_virtual_range.contains(&col.type_descriptor) {
                    return Some((col.type_descriptor, pfn));
                }
            }

            None
        })
        .collect::<Vec<_>>();

    possible_vtables.sort_by_key(|(td, _)| *td);

    let possible_vtables = possible_vtables.into_iter().fold(
        vec![],
        |mut v: Vec<(&TypeDescriptor, Vec<&Va>)>, (td, pfn)| {
            let Ok(td) = program.derva::<TypeDescriptor>(td) else {
                return v;
            };

            match v.last_mut() {
                Some((last_td, v)) if ptr::eq(*last_td, td) => {
                    v.push(pfn);
                }
                _ => {
                    v.push((td, vec![pfn]));
                }
            }

            v
        },
    );

    let map = possible_vtables
        .into_par_iter()
        .filter_map(|(td, v)| {
            let mangled = unsafe { CStr::from_ptr(td.name.as_ptr() as _).to_str().ok()? };

            let name = undname::demangle(mangled, undname::Flags::NAME_ONLY).ok()?;

            let mut vmts = v
                .into_iter()
                .map(|pfn| unsafe { UntypedVmt::new(pfn) })
                .collect::<Vec<_>>();

            // Won't panic - COLs are checked for validity beforehand.
            vmts.sort_by_cached_key(|vmt| vmt.col(program).unwrap().vmt_offset());

            Some((name.to_owned().into_boxed_str(), vmts.into_boxed_slice()))
        })
        .collect();

    Ok(map)
}

impl UntypedVmt {
    /// # Safety
    ///
    /// `ptr` must be pointing to the beginning of a vtable,
    /// preceded by a valid complete object locator pointer.
    const unsafe fn new(ptr: *const Va) -> Self {
        Self(ptr)
    }

    pub fn as_ptr<T>(self) -> *const T {
        self.0.cast()
    }

    pub fn col<'a, P>(self, program: P) -> Result<ClassCol<'a, P>, pelite::Error>
    where
        P: Pe<'a>,
    {
        // SAFETY: safe by contract of `UntypedVmt::new`.
        let col: &RTTICompleteObjectLocator = program
            .va_to_rva(unsafe { self.0.sub(1).read() })
            .and_then(|rva| program.derva(rva))?;

        Ok(ClassCol {
            program,
            inner: col,
        })
    }
}

impl<'a, P> ClassCol<'a, P>
where
    P: Pe<'a>,
{
    pub fn vmt_offset(&self) -> u32 {
        self.inner.offset
    }

    pub fn ctor_offset(&self) -> u32 {
        self.inner.cd_offset
    }

    pub fn type_descriptor(&self) -> Result<&'a TypeDescriptor, pelite::Error> {
        self.program.derva(self.inner.type_descriptor)
    }

    pub fn base_classes(&self) -> Result<BaseClasses<'a, P>, pelite::Error> {
        let class_descriptor: &RTTIClassHierarchyDescriptor =
            self.program.derva(self.inner.class_descriptor)?;

        let base_classes = self.program.derva_slice(
            class_descriptor.base_class_array,
            class_descriptor.num_base_classes as usize,
        )?;

        Ok(BaseClasses {
            program: self.program,
            attributes: class_descriptor.attributes,
            inner: base_classes,
        })
    }
}

impl<'a, P> BaseClasses<'a, P>
where
    P: Pe<'a>,
{
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn attributes(&self) -> u32 {
        self.attributes
    }

    pub fn get(&self, index: usize) -> Result<BaseClass<'a, P>, RttiError> {
        let base_class_rva = self.inner.get(index).ok_or(RttiError::Bounds)?;
        let base_class = self.program.derva(*base_class_rva)?;

        Ok(BaseClass {
            program: self.program,
            inner: base_class,
        })
    }
}

impl<'a, P> BaseClass<'a, P>
where
    P: Pe<'a>,
{
    pub fn extends_classes(&self) -> u32 {
        self.inner.num_contained_bases
    }

    pub fn pmd(&self) -> &PMD {
        &self.inner.pmd
    }

    pub fn attributes(&self) -> u32 {
        self.inner.attributes
    }

    pub fn type_descriptor(&self) -> Result<&'a TypeDescriptor, pelite::Error> {
        self.program.derva(self.inner.type_descriptor)
    }
}

unsafe impl Send for UntypedVmt {}

unsafe impl Sync for UntypedVmt {}
