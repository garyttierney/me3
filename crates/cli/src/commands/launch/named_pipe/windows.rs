#![cfg(windows)]

use std::{
    fs::File,
    io, mem,
    os::windows::{ffi::OsStrExt, io::FromRawHandle},
    path::Path,
};

use base64::{engine::general_purpose::URL_SAFE, Engine};
use tempfile::{NamedTempFile, TempPath};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE},
        Storage::FileSystem::PIPE_ACCESS_INBOUND,
        System::Pipes::{ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_WAIT},
    },
};

pub struct NamedPipe {
    handle: HANDLE,
}

impl NamedPipe {
    #[inline]
    pub fn create(path: &Path) -> io::Result<Self> {
        // https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-createnamedpipew#parameters
        // 1. The name must start with "\\.\pipe\".
        // 2. The rest may contain any characters other than the backslash.
        // 3. The combined length of the name must not exceed 256 characters.
        //   - Not sure if this includes the nul terminator or not? Better be safe.
        let mut name = Vec::with_capacity(256);

        name.extend(r"\\.\pipe\".encode_utf16());
        name.extend(path.components().flat_map(|c| c.as_os_str().encode_wide()));
        name.push(0);

        if name.len() > 256 {
            return Err(io::ErrorKind::InvalidFilename.into());
        }

        let handle = unsafe {
            CreateNamedPipeW(
                PCWSTR(name.as_ptr()),
                PIPE_ACCESS_INBOUND,
                PIPE_WAIT | PIPE_READMODE_BYTE,
                1,
                4096,
                4096,
                0,
                None,
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }

        Ok(Self { handle })
    }

    #[inline]
    pub fn create_temp<T, F: FnMut(Self) -> T>(mut f: F) -> io::Result<NamedTempFile<T>> {
        let mut rand_bytes = [0; 15];
        getrandom::fill(&mut rand_bytes)?;

        let path = URL_SAFE.encode(&rand_bytes);
        let file = Self::create(Path::new(&path))?;

        let mut temp_file = NamedTempFile::from_parts(f(file), TempPath::from_path(path));
        temp_file.disable_cleanup(true);

        Ok(temp_file)
    }

    #[inline]
    pub fn open(self) -> io::Result<File> {
        let handle = self.handle;
        mem::forget(self);

        unsafe {
            ConnectNamedPipe(handle, None)?;
        }

        unsafe { Ok(File::from_raw_handle(handle.0)) }
    }
}

impl Drop for NamedPipe {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

unsafe impl Send for NamedPipe {}

unsafe impl Sync for NamedPipe {}
