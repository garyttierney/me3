use std::{
    fs::File,
    io::{self, StdoutLock},
    sync::{Arc, Mutex, MutexGuard},
};

use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone)]
pub enum MakeWriterWrapper {
    Stdout,
    File(Arc<Mutex<File>>),
}

impl MakeWriterWrapper {
    #[inline]
    pub fn new(f: File) -> Self {
        Self::File(Arc::new(Mutex::new(f)))
    }

    #[inline]
    pub fn stdout() -> Self {
        Self::Stdout
    }
}

impl<'a> MakeWriter<'a> for MakeWriterWrapper {
    type Writer = WriterWrapper<'a>;

    #[inline]
    fn make_writer(&'a self) -> Self::Writer {
        match self {
            Self::Stdout => WriterWrapper::Stdout(io::stdout().lock()),
            Self::File(w) => WriterWrapper::File(w.lock().unwrap()),
        }
    }
}

#[derive(Debug)]
pub enum WriterWrapper<'a> {
    Stdout(StdoutLock<'a>),
    File(MutexGuard<'a, File>),
}

impl io::Write for WriterWrapper<'_> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Stdout(w) => w.write(buf),
            Self::File(w) => w.write(buf),
        }
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Stdout(w) => w.flush(),
            Self::File(w) => w.flush(),
        }
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        match self {
            Self::Stdout(w) => w.write_vectored(bufs),
            Self::File(w) => w.write_vectored(bufs),
        }
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        match self {
            Self::Stdout(w) => w.write_all(buf),
            Self::File(w) => w.write_all(buf),
        }
    }

    #[inline]
    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> io::Result<()> {
        match self {
            Self::Stdout(w) => w.write_fmt(fmt),
            Self::File(w) => w.write_fmt(fmt),
        }
    }
}
