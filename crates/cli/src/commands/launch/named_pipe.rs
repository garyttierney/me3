#![cfg(unix)]

use std::{
    ffi::CString,
    fs::File,
    io,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use libc::mkfifo;

pub fn open(path: &Path) -> io::Result<PathBuf> {
    let c_str = CString::new(path.as_os_str().as_bytes()).map_err(io::Error::other)?;

    if unsafe { mkfifo(c_str.as_ptr(), 0o666) != 0 } {
        return Err(io::Error::last_os_error());
    }

    Ok(path.to_path_buf())
}
