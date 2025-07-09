use std::mem;

use regex::bytes::Regex;
use thiserror::Error;
use tracing::debug;

use crate::{mapping::ArchiveOverrideMapping, pe, rtti};

pub type WwiseOpenFileByName =
    unsafe extern "C" fn(usize, *const u16, u64, usize, usize, usize) -> usize;

#[repr(C)]
struct FilePackageLowLevelIOBlockingVtable {
    _dtor: usize,
    _open_by_id: usize,
    open_by_name: WwiseOpenFileByName,
}

const PREFIXES: &[&str] = &["sd", "sd/enus", "sd/ja"];

/// Strip sd:/ and sd_dlc02:/ prefixes from the input string.
pub fn strip_prefix(input: &str) -> &str {
    let mut start = 0;
    loop {
        let mut found = false;
        for prefix in &["sd:/", "sd_dlc02:/"] {
            if input[start..].starts_with(prefix) {
                start += prefix.len();
                found = true;
                // Restart the loop once a prefix is removed.
                break;
            }
        }
        if !found {
            break;
        }
    }
    &input[start..]
}

#[repr(u32)]
pub enum AkOpenMode {
    Read = 0,
    Write = 1,
    WriteOverwrite = 2,
    ReadWrite = 3,
    ReadEbl = 10,
}

/// Tries to find an override for a sound archive entry.
pub fn find_override<'a>(
    mapping: &'a ArchiveOverrideMapping,
    input: &str,
) -> Option<(&'a str, &'a [u16])> {
    let input = strip_prefix(input);
    if input.ends_with(".wem") {
        let wem_path = format!("wem/{input}");
        if let Some(replacement) = get_override(mapping, &wem_path) {
            return Some(replacement);
        }

        // ER stores WEMs at wem/<first two digits of wemID>/wemID.wem so we need to check that
        // location too.
        let folder = input.split_at(2).0;
        let wem_path = format!("wem/{folder}/{input}");
        if let Some(replacement) = get_override(mapping, &wem_path) {
            return Some(replacement);
        }
    } else if let Some(replacement) = get_override(mapping, input) {
        return Some(replacement);
    }

    None
}

fn get_override<'a>(
    mapping: &'a ArchiveOverrideMapping,
    input: &str,
) -> Option<(&'a str, &'a [u16])> {
    for prefix in PREFIXES {
        let prefixed = format!("{prefix}/{input}");
        if let Some(replacement) = mapping.vfs_override(&prefixed) {
            return Some(replacement);
        }
    }
    None
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use crate::{mapping::ArchiveOverrideMapping, wwise::find_override};

    #[test]
    fn scan_directory_and_overrides() {
        let mut asset_mapping = ArchiveOverrideMapping::new().unwrap();

        let test_mod_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data/test-mod");
        asset_mapping.scan_directory(test_mod_dir).unwrap();

        assert!(
            find_override(&asset_mapping, "sd:/init.bnk").is_some(),
            "override for init.bnk was not found"
        );
        assert!(
            find_override(&asset_mapping, "sd:/1000519763.wem").is_some(),
            "override for sd:/1000519763.wem not found"
        );
        assert!(
            find_override(&asset_mapping, "sd:/485927883.wem").is_some(),
            "override for sd:/485927883.wem not found"
        );
    }
}

/// # Safety
/// [`pelite::pe64::PeView::module`] must be safe to call on `image_base`
pub unsafe fn find_wwise_open_file(
    image_base: *const u8,
) -> Result<WwiseOpenFileByName, FindError> {
    let rtti_result = unsafe {
        find_wwise_open_file_fn_by_rtti(image_base)
            .inspect_err(|e| debug!("DLMOW::IOHookBlocking RTTI scan error: {e}"))
    };

    if let Ok(result) = rtti_result {
        return Ok(result);
    }

    unsafe { find_wwise_open_file_fn_by_scan(image_base) }
}

unsafe fn find_wwise_open_file_fn_by_rtti(
    image_base: *const u8,
) -> Result<WwiseOpenFileByName, FindError> {
    let open_by_name = unsafe {
        rtti::find_vmt::<FilePackageLowLevelIOBlockingVtable>(image_base, "DLMOW::IOHookBlocking")
            .map(|vmt| vmt.as_ref().open_by_name)?
    };

    Ok(open_by_name)
}

unsafe fn find_wwise_open_file_fn_by_scan(
    image_base: *const u8,
) -> Result<WwiseOpenFileByName, FindError> {
    let [text] = unsafe { pe::sections(image_base, [".text"])? };

    let open_file_re = Regex::new(
        r"(?s-u)\xe8(.{4})\x83\xf8\x01(?:(?:\x74.)|(?:\x0f\x84.{4}))[\x48-\x4f]\x83[\xc0-\xc7]\x38[\x48-\x4f]\x83(?:(?:\x7d.)|(?:\xbd.{4}))\x08",
    )
    .unwrap();

    let call_disp32 = open_file_re
        .captures(text)
        .and_then(|c| c.iter().nth(1).flatten())
        .ok_or(FindError::Pattern)?
        .as_bytes();

    let call_bytes = <[u8; 4]>::try_from(call_disp32).unwrap();

    unsafe {
        Ok(mem::transmute(
            call_disp32
                .as_ptr_range()
                .end
                .offset(i32::from_le_bytes(call_bytes) as _),
        ))
    }
}

#[derive(Error, Debug)]
pub enum FindError {
    #[error("{0}")]
    Rtti(rtti::FindError),
    #[error("{0}")]
    PeSection(pe::SectionError),
    #[error("Pattern scan returned no matches")]
    Pattern,
}

#[derive(Error, Debug)]
#[error("Function timed out; last error: {0}")]
pub struct TimeoutError(FindError);

impl From<rtti::FindError> for FindError {
    fn from(value: rtti::FindError) -> Self {
        FindError::Rtti(value)
    }
}

impl From<pe::SectionError> for FindError {
    fn from(value: pe::SectionError) -> Self {
        FindError::PeSection(value)
    }
}
