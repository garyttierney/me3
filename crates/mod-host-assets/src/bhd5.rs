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

#[repr(C)]
pub struct Bhd5Holder {
    bhd_header: Option<NonNull<Bhd5Header>>,
    bucket_count: u32,
    buckets: Option<NonNull<u32>>,
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

impl Bhd5Holder {
    pub fn bhd_header(&self) -> Option<&Bhd5Header> {
        self.bhd_header.map(|ptr| unsafe { ptr.as_ref() })
    }

    /// # Safety
    ///
    /// The buffer pointed to by `contents` should be aligned and allocated
    /// with a [`DlStdAllocator`], so that it may be freed later.
    pub unsafe fn assign_bhd_contents(&mut self, contents: *mut Bhd5Header) {
        self.bhd_header = NonNull::new(contents);

        if let Some(contents) = &self.bhd_header {
            let header = unsafe { contents.as_ref() };

            let buckets = header.buckets();

            self.bucket_count = buckets.len() as u32;
            self.buckets = Some(buckets.cast());
        }
    }
}
