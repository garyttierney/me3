use std::{mem, ptr::NonNull, time};

use thiserror::Error;
use tracing::debug;
use windows::{
    core::{Error as WinError, PCSTR, PCWSTR},
    Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress},
};

use crate::{mapping::ArchiveOverrideMapping, pe, rtti};

type FileLocationResolver = *const *const u8;

type GetFileLocationResolver = extern "C" fn() -> Option<NonNull<FileLocationResolver>>;

pub type WwiseOpenFileByName = extern "C" fn(usize, *const u16, u64, usize, usize, usize) -> usize;

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
        if let Some(replacement) = mapping.get_override(&prefixed) {
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
        let mut asset_mapping = ArchiveOverrideMapping::default();

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

pub fn poll_wwise_open_file_fn(
    freq: time::Duration,
    timeout: time::Duration,
) -> Result<WwiseOpenFileByName, FindError> {
    let mut rtti_scan = Some(std::thread::spawn(|| unsafe {
        find_wwise_open_file_fn_by_rtti()
            .inspect_err(|e| debug!("DLMOW::FilePackageLowLevelIOBlocking RTTI scan error: {e}"))
    }));

    let start = time::Instant::now();

    let mut by_export_result = None;

    while time::Instant::now().checked_duration_since(start).unwrap() < timeout {
        if rtti_scan.as_ref().is_some_and(|t| t.is_finished()) {
            if let Some(Ok(open_file_fn)) = rtti_scan.take().and_then(|t| t.join().ok()) {
                debug!(
                    "WwiseOpenFileByName found at {:?} via RTTI",
                    open_file_fn as *const u8
                );

                return Ok(open_file_fn);
            }
        }

        by_export_result = Some(find_wwise_open_file_fn_by_export());

        if let &Some(Ok(open_file_fn)) = &by_export_result {
            debug!(
                "WwiseOpenFileByName found at {:?} via AK::StreamMgr::GetFileLocationResolver",
                open_file_fn as *const u8
            );

            return Ok(open_file_fn);
        }

        std::thread::sleep(freq);
    }

    if let Some(Err(e)) = by_export_result {
        debug!("poll_wwise_open_file_fn timed out; last error: {e}");
    }

    Err(FindError::TimedOut)
}

/// # Safety
/// [`pelite::pe64::PeView::module`] must be safe to call on `image_base`
unsafe fn find_wwise_open_file_fn_by_rtti() -> Result<WwiseOpenFileByName, FindError> {
    let image_base = unsafe { GetModuleHandleW(PCWSTR::null())?.0 as _ };

    // SAFETY: must be upheld by caller.
    let open_by_name = unsafe {
        rtti::find_vmt::<FilePackageLowLevelIOBlockingVtable>(
            image_base,
            "DLMOW::FilePackageLowLevelIOBlocking",
        )
        .map(|vmt| vmt.as_ref().open_by_name)?
    };

    Ok(open_by_name)
}

fn find_wwise_open_file_fn_by_export() -> Result<WwiseOpenFileByName, FindError> {
    let module_handle = unsafe { GetModuleHandleW(PCWSTR::null())? };

    let file_location_resolver = unsafe {
        // IAkFileLocationResolver* AK::StreamMgr::GetFileLocationResolver()
        const EXPORT_NAME: &str =
            "?GetFileLocationResolver@StreamMgr@AK@@YAPEAVIAkFileLocationResolver@12@XZ\0";

        let far_proc = GetProcAddress(module_handle, PCSTR::from_raw(EXPORT_NAME.as_ptr()))
            .ok_or(FindError::Export("AK::StreamMgr::GetFileLocationResolver"))?;

        mem::transmute::<_, GetFileLocationResolver>(far_proc)().ok_or(FindError::Uninit)?
    };

    // SAFETY: image base obtained from GetModuleHandleW that didn't fail.
    let [text, rdata] = unsafe { pe::sections(module_handle.0 as _, [".text", ".rdata"])? };

    let vtable_ptr = unsafe { file_location_resolver.read() };

    let vtable_end = vtable_ptr.wrapping_add(7);

    if !vtable_ptr.is_aligned()
        || !rdata.as_ptr_range().contains(&vtable_ptr.cast())
        || !rdata
            .as_ptr_range()
            .contains(&vtable_end.wrapping_byte_sub(1).cast())
    {
        return Err(FindError::Vtable);
    }

    let mut fn_ptr = vtable_ptr;

    while fn_ptr < vtable_end {
        // SAFETY: pointer is aligned and in ".rdata".
        if !text.as_ptr_range().contains(unsafe { &*fn_ptr }) {
            return Err(FindError::Vtable);
        }

        fn_ptr = fn_ptr.wrapping_add(1);
    }

    let wwise_open_file =
        unsafe { (*vtable_ptr.cast::<FilePackageLowLevelIOBlockingVtable>()).open_by_name };

    Ok(wwise_open_file)
}

#[derive(Error, Debug)]
pub enum FindError {
    #[error("{0}")]
    Rtti(rtti::FindError),
    #[error("{0}")]
    PeSection(pe::SectionError),
    #[error("Low level WINAPI error {0}")]
    Winapi(WinError),
    #[error("Export {0} not found")]
    Export(&'static str),
    #[error("FileLocationResolver is uninitialized")]
    Uninit,
    #[error("Virtual function table layout mismatch")]
    Vtable,
    #[error("Function timed out")]
    TimedOut,
}

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

impl From<WinError> for FindError {
    fn from(value: WinError) -> Self {
        FindError::Winapi(value)
    }
}
