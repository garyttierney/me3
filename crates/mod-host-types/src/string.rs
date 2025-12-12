//! DLBasicString with additional type checking using the provided encoding
//! discriminator.
//!
//! [`DlString`] instances can be read and written from existing structures, but not created.
//!
//! [`DLHashString`] instances cache its DLHash 32-bit hash using interior mutability.
//!
//! Thanks to Axi! for finding out the possible encoding tags.

use std::{
    ffi::OsStr,
    fmt,
    mem::{self, ManuallyDrop},
    ops::{Deref, DerefMut},
    ptr,
};

use me3_mod_protocol::Game;
use rdvec::{
    alloc::{Alloc, AllocatorAware},
    RawVec, Vec,
};
use thiserror::Error;

use crate::{alloc::DlStdAllocator, game::GAME};

mod msvc2012;
mod msvc2015;

pub struct DlEncoding;

impl DlEncoding {
    pub const UTF8: u8 = 0;
    pub const UTF16: u8 = 1;
    pub const ISO_8859: u8 = 2;
    pub const SJIS: u8 = 3;
    pub const EUC_JP: u8 = 4;
    pub const UTF32: u8 = 5;
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum DlCharacterSet {
    Utf8 = DlEncoding::UTF8,
    Utf16 = DlEncoding::UTF16,
    Iso8859 = DlEncoding::ISO_8859,
    ShiftJis = DlEncoding::SJIS,
    EucJp = DlEncoding::EUC_JP,
    Utf32 = DlEncoding::UTF32,
}

#[repr(C)]
struct RawCxxString<T> {
    inner: RawCxxStringUnion<T>,
    len: usize,
    cap: usize,
}

#[repr(C)]
union RawCxxStringUnion<T> {
    buf: [u8; 16],
    ptr: *mut T,
}

#[repr(C)]
pub union DlString<T: 'static, const E: u8> {
    msvc2012: ManuallyDrop<msvc2012::DlString<T, E>>,
    msvc2015: ManuallyDrop<msvc2015::DlString<T, E>>,
}

pub type DlUtf8String = DlString<u8, { DlEncoding::UTF8 }>;

pub type DlUtf16String = DlString<u16, { DlEncoding::UTF16 }>;
pub type DlUtf16HashString = DlHashString<u16, { DlEncoding::UTF16 }>;

pub type DlIso8859String = DlString<u8, { DlEncoding::ISO_8859 }>;

pub type DlSjisString = DlString<u8, { DlEncoding::SJIS }>;

pub type DlEucJpString = DlString<u8, { DlEncoding::EUC_JP }>;

pub type DlUtf32String = DlString<u32, { DlEncoding::UTF32 }>;

#[repr(C)]
#[derive(Clone, Debug)]
pub struct DlHashString<T: 'static, const E: u8> {
    _vtable: usize,
    string: DlString<T, E>,
    _hash: u32,
    _is_unhashed: bool,
}

#[repr(transparent)]
#[derive(Clone, Debug)]
pub struct TrustedDlString<T: 'static, const E: u8>(DlString<T, E>);

impl<T: 'static, const E: u8> DlString<T, E> {
    #[inline]
    fn new() -> Self {
        Self::new_in(Default::default())
    }

    #[inline]
    fn new_in(alloc: DlStdAllocator) -> Self {
        match *GAME {
            game if game < Game::Sekiro => Self {
                msvc2012: ManuallyDrop::new(msvc2012::DlString::new_in(alloc)),
            },
            _ => Self {
                msvc2015: ManuallyDrop::new(msvc2015::DlString::new_in(alloc)),
            },
        }
    }

    #[inline]
    pub fn encoding(&self) -> Result<DlCharacterSet, <DlCharacterSet as TryFrom<u8>>::Error> {
        self.raw_encoding().try_into()
    }

    #[inline]
    pub fn get(&self) -> Result<&TrustedDlString<T, E>, EncodingError> {
        if self.raw_encoding() == E {
            // SAFETY: transmuting into transparent wrapper.
            unsafe { Ok(mem::transmute(self)) }
        } else {
            Err(EncodingError::new(Self::EXPECTED, self.raw_encoding()))
        }
    }

    #[inline]
    pub fn get_mut(&mut self) -> Result<&mut TrustedDlString<T, E>, EncodingError> {
        if self.raw_encoding() == E {
            // SAFETY: transmuting into transparent wrapper.
            unsafe { Ok(mem::transmute(self)) }
        } else {
            Err(EncodingError::new(Self::EXPECTED, self.raw_encoding()))
        }
    }

    /// # Safety
    ///
    /// Only if the static encoding matches the encoding of `self`.
    #[inline]
    pub unsafe fn get_unchecked(&self) -> &TrustedDlString<T, E> {
        // SAFETY: transmuting into transparent wrapper.
        unsafe { mem::transmute(self) }
    }

    /// # Safety
    ///
    /// Only if the static encoding matches the encoding of `self`.
    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self) -> &mut T {
        // SAFETY: transmuting into transparent wrapper.
        unsafe { mem::transmute(self) }
    }

    #[inline]
    fn as_dyn(&self) -> &dyn Vec<T, Alloc = dyn Alloc<T>> {
        match *GAME {
            game if game < Game::Sekiro => unsafe { &*self.msvc2012 },
            _ => unsafe { &*self.msvc2015 },
        }
    }

    #[inline]
    fn as_mut_dyn(&mut self) -> &mut dyn Vec<T, Alloc = dyn Alloc<T>> {
        match *GAME {
            game if game < Game::Sekiro => unsafe { &mut *self.msvc2012 },
            _ => unsafe { &mut *self.msvc2015 },
        }
    }

    #[inline]
    fn raw_encoding(&self) -> u8 {
        match *GAME {
            game if game < Game::Sekiro => unsafe { self.msvc2012.encoding },
            _ => unsafe { self.msvc2015.encoding },
        }
    }

    const EXPECTED: DlCharacterSet = match DlCharacterSet::from_raw(E) {
        Ok(encoding) => encoding,
        Err(_undefined) => panic!("encoding not defined"),
    };
}

impl<T> RawCxxString<T> {
    #[inline]
    fn small_mode_capacity() -> usize {
        usize::saturating_sub(16 / mem::size_of::<T>(), 1)
    }

    #[inline]
    fn is_small_mode(&self) -> bool {
        self.cap <= Self::small_mode_capacity()
    }
}

unsafe impl<T> RawVec<T> for RawCxxString<T> {
    #[inline]
    fn as_ptr(&self) -> *const T {
        if self.is_small_mode() {
            unsafe { self.inner.buf.as_ptr() as *const T }
        } else {
            unsafe { self.inner.ptr }
        }
    }

    #[inline]
    fn as_mut_ptr(&mut self) -> *mut T {
        if self.is_small_mode() {
            unsafe { self.inner.buf.as_mut_ptr() as *mut T }
        } else {
            unsafe { self.inner.ptr }
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.len
    }

    #[inline]
    fn capacity(&self) -> usize {
        self.cap
    }

    #[inline]
    fn max_len(&self) -> usize {
        isize::MAX as usize / mem::size_of::<T>().min(1) - 1
    }

    #[inline]
    unsafe fn set_len(&mut self, new_len: usize) {
        // Write the nul terminator.
        unsafe {
            self.as_mut_ptr().add(new_len).write_bytes(0, 1);
        }

        self.len = new_len;
    }

    #[inline]
    unsafe fn set_buf(&mut self, new_buf: *mut [T]) {
        let new_capacity = new_buf.len();
        let small_capacity = RawCxxString::<T>::small_mode_capacity();

        self.cap = new_capacity;

        if ptr::addr_eq(new_buf, &raw const self.inner.buf) {
            return;
        }

        if small_capacity >= new_capacity {
            panic!("new capacity (is {new_capacity}) is too small (<= {small_capacity})");
        }

        // Write the nul terminator.
        unsafe {
            (new_buf as *mut T).add(self.len()).write_bytes(0, 1);
        }

        self.inner.ptr = new_buf as *mut T;
    }
}

unsafe impl<T: 'static, const E: u8> RawVec<T> for DlString<T, E> {
    #[inline]
    fn as_ptr(&self) -> *const T {
        self.as_dyn().as_ptr()
    }

    #[inline]
    fn as_mut_ptr(&mut self) -> *mut T {
        self.as_mut_dyn().as_mut_ptr()
    }

    #[inline]
    fn len(&self) -> usize {
        self.as_dyn().len()
    }

    #[inline]
    fn capacity(&self) -> usize {
        self.as_dyn().capacity()
    }

    #[inline]
    fn max_len(&self) -> usize {
        self.as_dyn().max_len()
    }

    #[inline]
    unsafe fn set_len(&mut self, new_len: usize) {
        unsafe {
            self.as_mut_dyn().set_len(new_len);
        }
    }

    #[inline]
    unsafe fn set_buf(&mut self, new_buf: *mut [T]) {
        unsafe {
            self.as_mut_dyn().set_buf(new_buf);
        }
    }
}

impl<T: 'static, const E: u8> AllocatorAware<T> for DlString<T, E> {
    type Alloc = dyn Alloc<T>;

    #[inline]
    fn allocator(&self) -> &Self::Alloc {
        self.as_dyn().allocator()
    }
}

impl<T, const E: u8> Clone for DlString<T, E>
where
    T: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        let mut cloned = match *GAME {
            game if game < Game::Sekiro => unsafe { Self::new_in(self.msvc2012.alloc) },
            _ => unsafe { Self::new_in(self.msvc2015.alloc) },
        };

        cloned.extend_from_slice(self.as_slice());

        cloned
    }
}

impl<T, const E: u8> Default for DlString<T, E> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Default for RawCxxString<T> {
    #[inline]
    fn default() -> Self {
        Self {
            inner: RawCxxStringUnion { buf: [0; 16] },
            len: 0,
            cap: RawCxxString::<T>::small_mode_capacity(),
        }
    }
}

impl<T: 'static, const E: u8> Drop for DlString<T, E> {
    fn drop(&mut self) {
        unsafe {
            let buf = self.as_buf();
            let elems = &raw mut *self.as_mut_slice();

            match *GAME {
                game if game < Game::Sekiro => self.msvc2012.raw = Default::default(),
                _ => self.msvc2015.raw = Default::default(),
            }

            elems.drop_in_place();
            self.allocator().dealloc(buf).unwrap();
        }
    }
}

#[derive(Error, Debug)]
#[error(
    "DlString encoding error; expected {:?} but got {:?}",
    expected, DlCharacterSet::try_from(*actual)
)]
pub struct EncodingError {
    expected: DlCharacterSet,
    actual: u8,
}

impl EncodingError {
    #[inline]
    const fn new(expected: DlCharacterSet, actual: u8) -> Self {
        Self { expected, actual }
    }
}

impl DlCharacterSet {
    #[inline]
    const fn from_raw(value: u8) -> Result<Self, u8> {
        match value {
            DlEncoding::UTF8 => Ok(DlCharacterSet::Utf8),
            DlEncoding::UTF16 => Ok(DlCharacterSet::Utf16),
            DlEncoding::ISO_8859 => Ok(DlCharacterSet::Iso8859),
            DlEncoding::SJIS => Ok(DlCharacterSet::ShiftJis),
            DlEncoding::EUC_JP => Ok(DlCharacterSet::EucJp),
            DlEncoding::UTF32 => Ok(DlCharacterSet::Utf32),
            value => Err(value),
        }
    }
}

impl TryFrom<u8> for DlCharacterSet {
    type Error = u8;

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_raw(value)
    }
}

impl<T, const E: u8> AsRef<DlString<T, E>> for TrustedDlString<T, E> {
    #[inline]
    fn as_ref(&self) -> &DlString<T, E> {
        &self.0
    }
}

impl<T, const E: u8> AsMut<DlString<T, E>> for TrustedDlString<T, E> {
    #[inline]
    fn as_mut(&mut self) -> &mut DlString<T, E> {
        &mut self.0
    }
}

impl<T, const E: u8> Deref for DlHashString<T, E> {
    type Target = DlString<T, E>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.string
    }
}

impl<T, const E: u8> DerefMut for DlHashString<T, E> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.string
    }
}

impl<T, const E: u8> Deref for TrustedDlString<T, E> {
    type Target = DlString<T, E>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, const E: u8> DerefMut for TrustedDlString<T, E> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl fmt::Display for TrustedDlString<u8, { DlEncoding::UTF8 }> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // SAFETY: `self` is encoded at least as a UTF-8 superset
        // as verified by checking the encoding.
        let os_str = unsafe { OsStr::from_encoded_bytes_unchecked(self.as_slice()) };

        f.write_str(&os_str.to_string_lossy())
    }
}

#[cfg(windows)]
impl fmt::Display for TrustedDlString<u16, { DlEncoding::UTF16 }> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::{ffi::OsString, os::windows::ffi::OsStringExt};

        let os_str = OsString::from_wide(self.as_slice());

        f.write_str(&os_str.to_string_lossy())
    }
}

impl<T: fmt::Debug, const E: u8> fmt::Debug for DlString<T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DlString")
            .field("len", &self.len())
            .field("cap", &self.capacity())
            .field("encoding", &self.encoding())
            .finish()
    }
}
