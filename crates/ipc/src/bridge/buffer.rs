use std::{
    hint,
    ops::{Deref, DerefMut},
    ptr::NonNull,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::bridge::rel::RelPtr;

/// Bipartite buffer implementation.
///
/// Strict MPSC (multiple producers, single consumer), `read` is an unsafe fn.
///
/// Wrapping is achieved by the fact every message is prefixed with its size
/// in a custom LEB128 format. Only the write pointer `w` can catch up to the read pointer.
/// The read pointer `r` must trail behind `w`.
///
/// All messages are aligned to 16 bytes. For messages too long to fit at the end, their size is
/// written as normal and the contents are written at position 0.
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
        // SAFETY: up to caller.
        self.buf = unsafe { RelPtr::new(buf, NonNull::from_ref(self).cast()) };

        // The end is not aligned to 16 bytes, so it can always fit a LEB but not a message.
        self.len = u32::saturating_sub(len & Self::ALIGN.wrapping_neg(), 1);
    }

    /// # Safety
    ///
    /// Only one thread is permitted to read from the buffer at a time.
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

        // Read the next message length.
        // Note how it's unconditionally at the read position if the buffer is non-empty.
        let len = unsafe { read_leb128(&mut cursor) };

        let mut new_pos =
            unsafe { u32::next_multiple_of(cursor.offset_from_unsigned(buf) as u32, Self::ALIGN) };

        if new_pos + len + LEB128_CAP >= self.len {
            // The message is at the start of the buffer.
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
            // Don't try to write messages over half the length of the buffer.
            return Err(WriteError::TooLarge(len));
        }
        let len = len as u32;

        let r = self.r.load(Ordering::Acquire);

        // Have to spin to make writes atomic.
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

        // Using LEB128_CAP to make sure that `w` cannot reach `r`.
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

/// Little endian base 128 implementation. Set high bit indicates the end.
#[inline]
const fn size_of_leb128(value: u32) -> u32 {
    match value {
        0 => 1,
        value => value.ilog(128) + 1,
    }
}

/// Little endian base 128 implementation. Set high bit indicates the end.
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

/// Little endian base 128 implementation. Set high bit indicates the end.
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
