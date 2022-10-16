use std::io::{ErrorKind, Read};

pub trait ReadFormatsExt {
    fn read_magic<const LENGTH: usize>(&mut self, expected: &[u8; LENGTH]) -> std::io::Result<()>;
}

impl<R: Read> ReadFormatsExt for R {
    #[inline]
    fn read_magic<const LENGTH: usize>(&mut self, expected: &[u8; LENGTH]) -> std::io::Result<()> {
        let mut buffer = [0u8; LENGTH];
        self.read_exact(&mut buffer)?;

        if &buffer == expected {
            Ok(())
        } else {
            Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "expected {:?} ({:#x?}), found {:?} ({:#x?})",
                    String::from_utf8_lossy(expected),
                    expected,
                    String::from_utf8_lossy(&buffer),
                    &buffer
                ),
            ))
        }
    }
}
