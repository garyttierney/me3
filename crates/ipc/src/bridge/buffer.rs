use std::{
    hint,
    ops::{Deref, DerefMut},
    ptr::NonNull,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::bridge::rel::RelPtr;

#[repr(C)]
pub struct BipBuffer {
    buf: RelPtr<u8>,
    len: u32,
    lock: CachePaddedAtomicU32,
    w: CachePaddedAtomicU32,
    r: CachePaddedAtomicU32,
}

#[repr(C, align(64))]
struct CachePaddedAtomicU32(AtomicU32);

#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    #[error("input too large (tried to write {0} bytes)")]
    TooLarge(usize),

    #[error("the buffer is full")]
    Full,
}

impl BipBuffer {
    const ALIGN: u32 = 16;

    pub const fn new() -> Self {
        let dangling = NonNull::dangling();
        Self {
            buf: unsafe { RelPtr::new(dangling, dangling.cast()) },
            len: 0,
            lock: CachePaddedAtomicU32::new(0),
            w: CachePaddedAtomicU32::new(0),
            r: CachePaddedAtomicU32::new(0),
        }
    }

    pub unsafe fn init(&mut self, buf: NonNull<u8>, len: u32) {
        self.buf = unsafe { RelPtr::new(buf, NonNull::from_ref(self).cast()) };
        self.len = u32::saturating_sub(len & Self::ALIGN.wrapping_neg(), 1);
    }

    #[inline]
    pub unsafe fn read<T, F: FnOnce(&mut [u8]) -> T>(&self, f: F) -> Option<T> {
        let r = self.r.load(Ordering::Relaxed);
        let w = self.w.load(Ordering::Acquire);

        if r == w {
            return None;
        }

        let buf = self.buf();
        let read_start = unsafe { buf.add(r as usize) };

        let mut cursor = read_start;
        let len = unsafe { read_leb128(&mut cursor) };

        let mut new_pos =
            unsafe { u32::next_multiple_of(cursor.offset_from_unsigned(buf) as u32, Self::ALIGN) };

        if new_pos + len + LEB128_CAP >= self.len {
            new_pos = 0;
        }

        cursor = unsafe { buf.add(new_pos as usize) };
        let bytes = unsafe { NonNull::slice_from_raw_parts(cursor, len as usize).as_mut() };

        let value = f(bytes);

        let new_r = new_pos + len;
        self.r.store(new_r, Ordering::Release);

        Some(value)
    }

    #[inline]
    pub fn write(&self, bytes: &[u8]) -> Result<(), WriteError> {
        let len = bytes.len();
        let leb_len = size_of_leb128(len as u32);

        let max_len = len + leb_len as usize + Self::ALIGN as usize - 1;
        if max_len > self.len as usize / 2 {
            return Err(WriteError::TooLarge(len));
        }
        let len = len as u32;

        let r = self.r.load(Ordering::Acquire);

        struct LockGuard<'a>(&'a AtomicU32);

        impl Drop for LockGuard<'_> {
            #[inline]
            fn drop(&mut self) {
                self.0.store(0, Ordering::Relaxed);
            }
        }

        let _guard = loop {
            if self
                .lock
                .compare_exchange_weak(0, 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                break LockGuard(&self.lock);
            }

            while self.lock.load(Ordering::Relaxed) == 1 {
                hint::spin_loop();
            }
        };

        let w = self.w.load(Ordering::Relaxed);

        let buf = self.buf();
        let write_start = unsafe { buf.add(w as usize) };

        let mut new_pos = (w + leb_len).next_multiple_of(Self::ALIGN);
        let mut w_left_of_r = w < r;

        if new_pos + len + LEB128_CAP >= self.len {
            new_pos = 0;
            w_left_of_r = true;
        }

        if w_left_of_r && new_pos + len + LEB128_CAP >= r {
            return Err(WriteError::Full);
        }

        unsafe {
            write_leb128(write_start, len);
        }

        unsafe {
            NonNull::copy_to_nonoverlapping(
                NonNull::from_ref(bytes).cast(),
                buf.add(new_pos as usize),
                bytes.len(),
            );
        };

        self.w.store(new_pos + len, Ordering::Release);

        Ok(())
    }

    #[inline]
    fn buf(&self) -> NonNull<u8> {
        unsafe { self.buf.get(NonNull::from_ref(self).cast()) }
    }
}

const LEB128_CAP: u32 = size_of_leb128(u32::MAX);

#[inline]
const fn size_of_leb128(value: u32) -> u32 {
    match value {
        0 => 1,
        value => value.ilog(128) + 1,
    }
}

#[inline]
unsafe fn read_leb128(cursor: &mut NonNull<u8>) -> u32 {
    let mut x = 0u32;
    let mut pos = 0u32;
    while pos < u32::BITS {
        let byte = unsafe {
            let byte = cursor.read();
            *cursor = cursor.add(1);
            byte
        };
        if byte & 0x80 != 0 {
            return x | ((byte & 0x7f) as u32).wrapping_shl(pos);
        }
        x |= (byte as u32).wrapping_shl(pos);
        pos = pos.wrapping_add(7);
    }
    x
}

#[inline]
unsafe fn write_leb128(mut cursor: NonNull<u8>, mut x: u32) {
    loop {
        let byte = unsafe {
            let byte = cursor.as_mut();
            cursor = cursor.add(1);
            byte
        };
        if x <= 0x7f {
            *byte = x as u8 | 0x80;
            return;
        }
        *byte = (x & 0x7f) as u8;
        x >>= 7;
    }
}

impl CachePaddedAtomicU32 {
    const fn new(value: u32) -> Self {
        Self(AtomicU32::new(value))
    }
}

impl Deref for CachePaddedAtomicU32 {
    type Target = AtomicU32;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CachePaddedAtomicU32 {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
