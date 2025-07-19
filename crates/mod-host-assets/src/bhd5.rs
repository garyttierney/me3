use std::{ptr::NonNull, slice};

#[repr(C)]
pub struct Bhd5Header {
    magic: [u8; 4],
    endianness: u8,
    _unk05: [u8; 7],
    file_size: u32,
    bucket_count: u32,
    bucket_offset: u32,
    salt_length: u32,
    salt: [u8; 0],
}

impl Bhd5Header {
    pub fn file_size(&self) -> u32 {
        self.file_size
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(&raw const *self as _, self.file_size as usize) }
    }

    pub fn buckets(&self) -> NonNull<[u32]> {
        unsafe {
            NonNull::slice_from_raw_parts(
                NonNull::from(self)
                    .byte_add(self.bucket_offset as usize)
                    .cast(),
                self.bucket_count as usize,
            )
        }
    }
}
