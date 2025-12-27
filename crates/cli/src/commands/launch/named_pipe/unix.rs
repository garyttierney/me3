#![cfg(unix)]

use std::{
    ffi::CString,
    fs::File,
    io,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use libc::mkfifo;
use tempfile::NamedTempFile;

pub struct NamedPipe {
    path: PathBuf,
}

impl NamedPipe {
    #[inline]
    pub fn create(path: &Path) -> io::Result<Self> {
        let c_str = CString::new(path.as_os_str().as_bytes()).map_err(io::Error::other)?;

        if unsafe { mkfifo(c_str.as_ptr(), 0o666) != 0 } {
            return Err(io::Error::last_os_error());
        }

        let path = path.to_path_buf();
        Ok(Self { path })
    }

    #[inline]
    pub fn create_temp<T, F: FnMut(Self) -> T>(mut f: F) -> io::Result<NamedTempFile<T>> {
        tempfile::Builder::new()
            .rand_bytes(6)
            .make(|path| NamedPipe::create(path).map(&mut f))
    }

    #[inline]
    pub fn open(self) -> io::Result<File> {
        let path = self.path;
        File::open(&path)
    }
}
