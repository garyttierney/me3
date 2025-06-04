use std::{mem, ops::Range, ptr::NonNull, slice};

use pelite::pe::msvc::{RTTICompleteObjectLocator, TypeDescriptor};
use thiserror::Error;

use crate::pe;

/// # Safety
/// Virtual method table must be representable as `T`.
///
/// [`pelite::pe64::PeView::module`] must be safe to call on `image_base`
pub unsafe fn find_vmt<T>(image_base: *const u8, type_name: &str) -> Result<NonNull<T>, FindError> {
    // SAFETY: must be upheld by caller.
    let [text, rdata] = unsafe { pe::sections(image_base, [".text", ".rdata"])? };
    let [data_vrange] = unsafe { pe::section_vranges(image_base, [".data"])? };

    const SIZE: usize = mem::size_of::<*const RTTICompleteObjectLocator>();
    const ALIGNMENT: usize = mem::align_of::<*const RTTICompleteObjectLocator>();

    // https://learn.microsoft.com/en-us/cpp/error-messages/compiler-warnings/compiler-warning-level-1-c4503
    const MAX_DECORATED_LEN: usize = 4096;

    let mut check_next_for_vmt = {
        let unqualified = unqualify_type_name(type_name);
        let mut undecorated = String::with_capacity(MAX_DECORATED_LEN);

        move |rdata_ptr: &mut *const u8| {
            *rdata_ptr = rdata_ptr.wrapping_byte_add(SIZE);

            let vmt = rdata_ptr.wrapping_byte_add(SIZE);

            // SAFETY: pointer is aligned and non-null.
            let first_fn = unsafe { vmt.cast::<*const u8>().read() };

            // Confirm that the first pointer after the COL is a function pointer.
            if !text.as_ptr_range().contains(&first_fn) {
                return None;
            }

            // SAFETY: pointer is aligned and non-null.
            let col = unsafe { rdata_ptr.cast::<*const RTTICompleteObjectLocator>().read() };

            // Move pointer forward since the next pointer is a function pointer.
            *rdata_ptr = vmt;

            if !col.is_aligned() || !rdata.as_ptr_range().contains(&col.cast()) {
                return None;
            }

            // SAFETY: pointer is aligned, non-null and references read-only memory.
            let RTTICompleteObjectLocator {
                offset,
                cd_offset,
                type_descriptor,
                ..
            } = unsafe { &*col };

            if *offset != 0 || *cd_offset != 0 || !data_vrange.contains(type_descriptor) {
                return None;
            }

            // Inside the image because `*type_descriptor` is in ".data".
            let type_desc =
                image_base.wrapping_byte_add(*type_descriptor as _) as *const TypeDescriptor;

            if !type_desc.is_aligned() {
                return None;
            }

            let max_len = mem::size_of::<TypeDescriptor>()
                .wrapping_add(data_vrange.end.wrapping_sub(*type_descriptor) as usize)
                .min(MAX_DECORATED_LEN);

            // SAFETY: `type_desc` is aligned and non-null and is within ".data" with `max_len`.
            let decorated = unsafe { find_type_name(type_desc, unqualified, max_len)? };

            undname::demangle_into(decorated, undname::Flags::NAME_ONLY, &mut undecorated).ok()?;

            undecorated
                .eq_ignore_ascii_case(type_name)
                .then_some(*rdata_ptr)
        }
    };

    let Range { start, end } = rdata.as_ptr_range();

    let mut rdata_ptr =
        start.wrapping_byte_offset(start.align_offset(ALIGNMENT) as isize - SIZE as isize);

    // One less than actual because a COL must be followed by at least one function pointer.
    let rdata_end = end.wrapping_byte_sub(SIZE);

    while rdata_ptr < rdata_end {
        if let Some(vmt) = check_next_for_vmt(&mut rdata_ptr) {
            return Ok(NonNull::new(vmt as *mut T).unwrap());
        }
    }

    Err(FindError::Instance)
}

fn unqualify_type_name(type_name: &str) -> &str {
    type_name.rsplit_once(':').unwrap_or(("", type_name)).1
}

/// # Safety
/// - `desc` must be aligned and non-null.
/// - the total size up to `max_len` must not cross past the allocated object.
/// - [`slice::from_raw_parts`] must be safe to call on the resulting range.
unsafe fn find_type_name(
    desc: *const TypeDescriptor,
    type_name: &str,
    max_len: usize,
) -> Option<&'static str> {
    let name_bytes = type_name.as_bytes().iter().cloned().fuse();
    let mut next_name_byte = name_bytes.clone();

    let ptr = (*desc).name.as_ptr();
    let mut len = usize::wrapping_neg(1);

    loop {
        len = len.wrapping_add(1);

        if len == max_len {
            break;
        }

        // SAFETY: safe as long as it is in the same allocated object.
        let next_byte = unsafe { *ptr.add(len) };

        // Nul terminator, break and check if whole type name was matched.
        if next_byte == 0 {
            break;
        }

        // Check for non-ASCII contents.
        if next_byte > b'\x7f' {
            return None;
        }

        if next_byte != next_name_byte.next().unwrap_or(next_byte) {
            next_name_byte = name_bytes.clone();
        }
    }

    // Check if whole type name was matched.
    // SAFETY: `slice::from_raw_parts` must be safe by preconditions.
    // `str::from_utf8_unchecked` is safe because all characters are validated ASCII.
    match next_name_byte.next() {
        None => unsafe { Some(str::from_utf8_unchecked(slice::from_raw_parts(ptr, len))) },
        Some(_) => None,
    }
}

#[derive(Error, Debug)]
pub enum FindError {
    #[error("{0}")]
    PeSection(pe::SectionError),
    #[error("DlDeviceManager instance not found")]
    Instance,
}

impl From<pe::SectionError> for FindError {
    fn from(value: pe::SectionError) -> Self {
        FindError::PeSection(value)
    }
}
