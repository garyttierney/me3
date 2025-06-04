//! DLBasicString with additional type checking using the provided encoding
//! discriminator.
//!
//! [`DlString`] instances can be read and written from existing structures, but not created.
//!
//! [`DLHashString`] instances cache its DLHash 32-bit hash using interior mutability.
//!
//! Thanks to Axi! for finding out the possible encoding tags.

use core::fmt;
use std::{
    ffi::OsStr,
    mem,
    ops::{Deref, DerefMut},
};

use cxx_stl::{
    alloc::CxxProxy,
    string::{CxxNarrowString, CxxUtf16String, CxxUtf32String, CxxUtf8String},
};
use thiserror::Error;

use crate::alloc::DlStdAllocator;

const UTF8: u8 = 0;
const UTF16: u8 = 1;
const ISO_8859: u8 = 2;
const SJIS: u8 = 3;
const EUC_JP: u8 = 4;
const UTF32: u8 = 5;

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum DlCharacterSet {
    Utf8 = UTF8,
    Utf16 = UTF16,
    Iso8859 = ISO_8859,
    ShiftJis = SJIS,
    EucJp = EUC_JP,
    Utf32 = UTF32,
}

pub type DlUtf8String<A = DlStdAllocator> = DlString<CxxUtf8String<A>, { UTF8 }>;

pub type DlUtf16String<A = DlStdAllocator> = DlString<CxxUtf16String<A>, { UTF16 }>;
pub type DlUtf16HashString<A = DlStdAllocator> = DlHashString<CxxUtf16String<A>, { UTF16 }>;

pub type DlIso8859String<A = DlStdAllocator> = DlString<CxxNarrowString<A>, { ISO_8859 }>;

pub type DlSjisString<A = DlStdAllocator> = DlString<CxxNarrowString<A>, { SJIS }>;

pub type DlEucJpString<A = DlStdAllocator> = DlString<CxxNarrowString<A>, { EUC_JP }>;

pub type DlUtf32String<A = DlStdAllocator> = DlString<CxxUtf32String<A>, { UTF32 }>;

#[repr(C)]
#[derive(Clone)]
pub struct DlString<T, const E: u8> {
    inner: T,
    encoding: u8,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct DlHashString<T, const E: u8> {
    _vtable: usize,
    string: DlString<T, E>,
    _hash: u32,
    _is_unhashed: bool,
}

#[repr(transparent)]
#[derive(Clone, Debug)]
pub struct TrustedDlString<T, const E: u8>(DlString<T, E>);

impl<T, const E: u8> DlString<T, E> {
    pub fn encoding(&self) -> Result<DlCharacterSet, <DlCharacterSet as TryFrom<u8>>::Error> {
        self.encoding.try_into()
    }

    pub fn get(&self) -> Result<&TrustedDlString<T, E>, EncodingError> {
        if self.encoding == E {
            // SAFETY: transmuting into transparent wrapper.
            unsafe { Ok(mem::transmute(self)) }
        } else {
            Err(EncodingError::new(Self::EXPECTED, self.encoding))
        }
    }

    pub fn get_mut(&mut self) -> Result<&mut TrustedDlString<T, E>, EncodingError> {
        if self.encoding == E {
            // SAFETY: transmuting into transparent wrapper.
            unsafe { Ok(mem::transmute(self)) }
        } else {
            Err(EncodingError::new(Self::EXPECTED, self.encoding))
        }
    }

    /// # Safety
    /// Only if the static encoding matches the encoding of `self`.
    pub unsafe fn get_unchecked(&self) -> &TrustedDlString<T, E> {
        // SAFETY: transmuting into transparent wrapper.
        unsafe { mem::transmute(self) }
    }

    /// # Safety
    /// Only if the static encoding matches the encoding of `self`.
    pub unsafe fn get_unchecked_mut(&mut self) -> &mut T {
        // SAFETY: transmuting into transparent wrapper.
        unsafe { mem::transmute(self) }
    }

    const EXPECTED: DlCharacterSet = match DlCharacterSet::from_raw(E) {
        Ok(encoding) => encoding,
        Err(_undefined) => panic!("encoding not defined"),
    };
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
    const fn new(expected: DlCharacterSet, actual: u8) -> Self {
        Self { expected, actual }
    }
}

impl DlCharacterSet {
    const fn from_raw(value: u8) -> Result<Self, u8> {
        match value {
            UTF8 => Ok(DlCharacterSet::Utf8),
            UTF16 => Ok(DlCharacterSet::Utf16),
            ISO_8859 => Ok(DlCharacterSet::Iso8859),
            SJIS => Ok(DlCharacterSet::ShiftJis),
            EUC_JP => Ok(DlCharacterSet::EucJp),
            UTF32 => Ok(DlCharacterSet::Utf32),
            value => Err(value),
        }
    }
}

impl TryFrom<u8> for DlCharacterSet {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_raw(value)
    }
}

impl<T, const E: u8> AsRef<DlString<T, E>> for TrustedDlString<T, E> {
    fn as_ref(&self) -> &DlString<T, E> {
        &self.0
    }
}

impl<T, const E: u8> AsMut<DlString<T, E>> for TrustedDlString<T, E> {
    fn as_mut(&mut self) -> &mut DlString<T, E> {
        &mut self.0
    }
}

impl<T, const E: u8> Deref for DlHashString<T, E> {
    type Target = DlString<T, E>;

    fn deref(&self) -> &Self::Target {
        &self.string
    }
}

impl<T, const E: u8> DerefMut for DlHashString<T, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.string
    }
}

impl<T, const E: u8> Deref for TrustedDlString<T, E> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0.inner
    }
}

impl<T, const E: u8> DerefMut for TrustedDlString<T, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.inner
    }
}

impl<A: CxxProxy> fmt::Display for TrustedDlString<CxxUtf8String<A>, { UTF8 }> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // SAFETY: `self` is encoded at least as a UTF-8 superset
        // as verified by checking the encoding.
        let os_str = unsafe { OsStr::from_encoded_bytes_unchecked(self.as_bytes()) };

        f.write_str(&os_str.to_string_lossy())
    }
}

#[cfg(windows)]
impl<A: CxxProxy> fmt::Display for TrustedDlString<CxxUtf16String<A>, { UTF16 }> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::{ffi::OsString, os::windows::ffi::OsStringExt};

        let os_str = OsString::from_wide(self.as_bytes());

        f.write_str(&os_str.to_string_lossy())
    }
}

impl<T: fmt::Debug, const E: u8> fmt::Debug for DlString<T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DlString")
            .field("inner", &self.inner)
            .field("encoding", &self.encoding())
            .finish()
    }
}
