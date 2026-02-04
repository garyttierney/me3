use std::{
    borrow::Cow,
    ffi::OsString,
    os::windows::prelude::{OsStrExt as _, OsStringExt as _},
    path::{Path, PathBuf},
    sync::OnceLock,
};

use windows::{
    core::s,
    Win32::{
        Foundation::MAX_PATH,
        Globalization::WideCharToMultiByte,
        System::{
            LibraryLoader::{GetModuleHandleA, GetProcAddress},
            Memory::{GetProcessHeap, HeapFree, HEAP_FLAGS},
        },
    },
};

use crate::mapping::VfsOverrideMappingError;

type WineGetDosFileName = unsafe extern "C" fn(path: *const u8) -> *mut u16;
static WINE_GET_DOS_FILE_NAME_PTR: OnceLock<Option<WineGetDosFileName>> = OnceLock::new();

/// Ensures that any paths that refer to "host" paths under a compatibility layer like Proton/WINE
/// are normalized to DOS paths.
pub fn normalize_dos_path(path: &Path) -> Result<Cow<'_, Path>, VfsOverrideMappingError> {
    let wine_get_dos_file_name_ptr = WINE_GET_DOS_FILE_NAME_PTR.get_or_init(|| {
        let kernel32 = unsafe { GetModuleHandleA(s!("kernel32.dll")).ok() };

        kernel32.and_then(|k32| unsafe {
            std::mem::transmute(GetProcAddress(k32, s!("wine_get_dos_file_name")))
        })
    });

    // If we're not running under WINE immediately return the path as is.
    let Some(wine_get_dos_file_name) = wine_get_dos_file_name_ptr else {
        return Ok(Cow::Borrowed(path));
    };

    // We have a Unix path, so before we pass it to the wineserver we have to
    // convert it from UTF-16 into the encoding specified by the locale of the host system.
    // WINE implements an extension to Windows code pages that allows us to re-encode the string
    // into the format needed using Windows `WideCharToMultiByte` API. See also:
    // <https://github.com/wine-mirror/wine/blob/master/programs/winepath/winepath.c>

    const CP_UNIXCP: u32 = 65010; // WINE extension <https://github.com/wine-mirror/wine/blob/e53db200ca08f0aeb196617fa0238a776be2b7f8/include/winnls.h#L369>
    let os_path: Vec<u16> = path.as_os_str().encode_wide().collect();

    // SAFETY: os_path is a buffer of a known size.
    let unix_encoded_path_len =
        unsafe { WideCharToMultiByte(CP_UNIXCP, 0, &os_path, None, None, None) };

    if unix_encoded_path_len <= 0 {
        return Err(VfsOverrideMappingError::Compatibility);
    }

    let mut unix_encoded_path = vec![0u8; unix_encoded_path_len as usize];
    unsafe {
        WideCharToMultiByte(
            CP_UNIXCP,
            0,
            &os_path,
            Some(&mut unix_encoded_path),
            None,
            None,
        )
    };

    let dos_path_ptr = unsafe { wine_get_dos_file_name(unix_encoded_path.as_ptr() as *const _) };

    if dos_path_ptr.is_null() {
        return Err(VfsOverrideMappingError::Compatibility);
    }

    // SAFETY: dos_path_ptr is non-null
    let normalized = unsafe {
        let dos_path_len = libc::wcsnlen(dos_path_ptr as *const _, MAX_PATH as usize);
        let dos_path = std::slice::from_raw_parts(dos_path_ptr, dos_path_len);
        let normalized = PathBuf::from(OsString::from_wide(dos_path));

        // wineserver will allocate the result into the process heap and return us the pointer.
        let _ = HeapFree(
            GetProcessHeap().expect("must exist"),
            HEAP_FLAGS::default(),
            Some(dos_path_ptr as *const _),
        );

        normalized
    };

    Ok(Cow::Owned(normalized))
}
