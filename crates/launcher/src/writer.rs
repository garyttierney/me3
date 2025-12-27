use std::{
    fs::File,
    io,
    sync::{Arc, Mutex, MutexGuard},
};

use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone)]
pub struct MakeWriterWrapper {
    inner: Arc<Mutex<File>>,
}

impl MakeWriterWrapper {
    #[inline]
    pub fn new(f: File) -> Self {
        Self {
            inner: Arc::new(Mutex::new(f)),
        }
    }
}

impl<'a> MakeWriter<'a> for MakeWriterWrapper {
    type Writer = WriterWrapper<'a>;

    #[inline]
    fn make_writer(&'a self) -> Self::Writer {
        WriterWrapper(self.inner.lock().unwrap())
    }
}

#[derive(Debug)]
pub struct WriterWrapper<'a>(MutexGuard<'a, File>);

impl io::Write for WriterWrapper<'_> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        self.0.write_vectored(bufs)
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.0.write_all(buf)
    }

    #[inline]
    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> io::Result<()> {
        self.0.write_fmt(args)
    }
}
