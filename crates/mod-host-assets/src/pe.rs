use std::ops::Range;

use pelite::pe::{Pe, PeView};
use thiserror::Error;

/// # Safety
/// [`pelite::pe64::PeView::module`] must be safe to call on `image_base`
pub unsafe fn sections<const N: usize>(
    image_base: *const u8,
    section_names: [&'static str; N],
) -> Result<[&'static [u8]; N], SectionError> {
    // SAFETY: must be upheld by caller.
    let pe = unsafe { PeView::module(image_base) };
    let sections = pe.section_headers();

    let mut result = [[].as_slice(); N];

    for (i, name) in section_names.into_iter().enumerate() {
        result[i] = sections
            .by_name(name)
            .and_then(|s| pe.get_section_bytes(s).ok())
            .ok_or(SectionError {
                section: name,
                image_base,
            })?;
    }

    Ok(result)
}

/// # Safety
/// [`pelite::pe64::PeView::module`] must be safe to call on `image_base`
pub unsafe fn section_vranges<const N: usize>(
    image_base: *const u8,
    section_names: [&'static str; N],
) -> Result<[Range<u32>; N], SectionError> {
    // SAFETY: must be upheld by caller.
    let pe = unsafe { PeView::module(image_base) };
    let sections = pe.section_headers();

    let mut result = std::array::from_fn(|_| 0..0);

    for (i, name) in section_names.into_iter().enumerate() {
        result[i] = sections
            .by_name(name)
            .map(|s| s.virtual_range())
            .ok_or(SectionError {
                section: name,
                image_base,
            })?;
    }

    Ok(result)
}

#[derive(Debug, Error)]
#[error("section \"{section}\" not found in module with base address {image_base:#016x?}")]
pub struct SectionError {
    section: &'static str,
    image_base: *const u8,
}

unsafe impl Send for SectionError {}

unsafe impl Sync for SectionError {}
