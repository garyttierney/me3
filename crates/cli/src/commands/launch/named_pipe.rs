mod unix;
mod windows;

use std::{fs::File, io};

use tempfile::NamedTempFile;
#[cfg(unix)]
use unix::NamedPipe as OsNamedPipe;
#[cfg(windows)]
use windows::NamedPipe as OsNamedPipe;

pub struct NamedPipe(OsNamedPipe);

impl NamedPipe {
    pub fn create() -> io::Result<NamedTempFile<Self>> {
        OsNamedPipe::create_temp(Self)
    }

    pub fn open(self) -> io::Result<File> {
        self.0.open()
    }
}
