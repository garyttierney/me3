//! DLBasicString with additional type checking using the provided encoding
//! discriminator.
//!
//! [`DLString`] instances can be read and written from existing structures, but not created.
//!
//! [`DLHashString`] instances cache its DLHash 32-bit hash using interior mutability.
//!
//! Thanks to Axi! for finding out the possible encoding tags.

use cxx_stl::{
    alloc::CxxProxy,
    string::{CxxNarrowString, CxxUtf16String, CxxUtf32String, CxxUtf8String},
};
use thiserror::Error;

const UTF8: u8 = 0;
const UTF16: u8 = 1;
const ISO_8859: u8 = 2;
const SJIS: u8 = 3;
const EUC_JP: u8 = 4;
const UTF32: u8 = 5;

#[repr(u8)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug)]
pub enum DLStringEncoding {
    UTF8 = UTF8,
    UTF16 = UTF16,
    ISO_8859 = ISO_8859,
    SJIS = SJIS,
    EUC_JP = EUC_JP,
    UTF32 = UTF32,
}

pub type DLStringUtf8<A> = DLString<CxxUtf8String<A>, { UTF8 }>;

pub type DLStringUtf16<A> = DLString<CxxUtf16String<A>, { UTF16 }>;

pub type DLStringIso8859<A> = DLString<CxxNarrowString<A>, { ISO_8859 }>;

pub type DLStringSjis<A> = DLString<CxxNarrowString<A>, { SJIS }>;

pub type DLStringEucJP<A> = DLString<CxxNarrowString<A>, { EUC_JP }>;

pub type DLStringUtf32<A> = DLString<CxxUtf32String<A>, { UTF32 }>;

#[derive(Clone)]
pub struct DLString<T, const E: u8> {
    inner: T,
    encoding: u8,
}

impl<T, const E: u8> DLString<T, E> {
    pub fn encoding(&self) -> Result<DLStringEncoding, <DLStringEncoding as TryFrom<u8>>::Error> {
        self.encoding.try_into()
    }

    pub fn get(&self) -> Result<&T, EncodingError> {
        if self.encoding == E {
            Ok(&self.inner)
        } else {
            Err(EncodingError::new(Self::EXPECTED, self.encoding))
        }
    }

    pub fn get_mut(&mut self) -> Result<&mut T, EncodingError> {
        if self.encoding == E {
            Ok(&mut self.inner)
        } else {
            Err(EncodingError::new(Self::EXPECTED, self.encoding))
        }
    }

    pub unsafe fn get_unchecked(&self) -> &T {
        &self.inner
    }

    pub unsafe fn get_unchecked_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    const EXPECTED: DLStringEncoding = match DLStringEncoding::from_raw(E) {
        Ok(encoding) => encoding,
        Err(_undefined) => panic!("encoding not defined"),
    };
}

#[cfg(windows)]
impl<A: CxxProxy> DLStringUtf16<A> {
    pub fn decode(&self) -> Result<String, EncodingError> {
        use std::{ffi::OsString, os::windows::ffi::OsStringExt};

        Ok(OsString::from_wide(self.get()?.as_bytes())
            .to_string_lossy()
            .into_owned())
    }

    pub fn encode<T: AsRef<str>>(&mut self, s: T) -> Result<(), EncodingError> {
        let inner = self.get_mut()?;

        inner.clear();
        inner.extend(s.as_ref().encode_utf16());

        Ok(())
    }
}

#[derive(Error, Debug)]
#[error(
    "DLString encoding error; expected {:?} but got {:?}",
    expected, DLStringEncoding::try_from(*actual)
)]
pub struct EncodingError {
    expected: DLStringEncoding,
    actual: u8,
}

impl EncodingError {
    const fn new(expected: DLStringEncoding, actual: u8) -> Self {
        Self { expected, actual }
    }
}

impl DLStringEncoding {
    const fn from_raw(value: u8) -> Result<Self, u8> {
        match value {
            UTF8 => Ok(DLStringEncoding::UTF8),
            UTF16 => Ok(DLStringEncoding::UTF16),
            ISO_8859 => Ok(DLStringEncoding::ISO_8859),
            SJIS => Ok(DLStringEncoding::SJIS),
            EUC_JP => Ok(DLStringEncoding::EUC_JP),
            UTF32 => Ok(DLStringEncoding::UTF32),
            value => Err(value),
        }
    }
}

impl TryFrom<u8> for DLStringEncoding {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_raw(value)
    }
}
