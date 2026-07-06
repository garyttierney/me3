//! Thin C string types that are ffi-compatible with thin references.

use std::{
    char::{self, DecodeUtf16Error},
    ffi::{c_char, CStr},
    fmt,
    marker::PhantomData,
    ops::Deref,
    ptr::NonNull,
};

/// A borrowed nul-terminated string of unspecified encoding with the same ABI as [`NonNull`].
///
/// The [`Debug`] implementation for this type interprets the string as potentially invalid UTF-8.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct ThinCStr<'a>(NonNull<c_char>, PhantomData<&'a ()>);

impl<'a> ThinCStr<'a> {
    /// Construct a [`ThinCStr`] from a [`CStr`] ref.
    pub fn from_cstr(cstr: &'a CStr) -> Self {
        Self(NonNull::from_ref(cstr).cast(), PhantomData)
    }

    /// Construct a [`ThinCStr`] from a pointer to a nul-terminated C string.
    ///
    /// # Safety
    ///
    /// Has the same invariants as [`CStr::from_ptr`].
    pub unsafe fn from_ptr(ptr: NonNull<c_char>) -> Self {
        Self(ptr, PhantomData)
    }
}

impl<'a> From<&'a CStr> for ThinCStr<'a> {
    fn from(value: &'a CStr) -> Self {
        Self::from_cstr(value)
    }
}

impl<'a> From<ThinCStr<'a>> for &'a CStr {
    fn from(value: ThinCStr<'a>) -> Self {
        // SAFETY: By the invariants of the type
        unsafe { CStr::from_ptr(value.0.as_ptr()) }
    }
}

impl AsRef<CStr> for ThinCStr<'_> {
    fn as_ref(&self) -> &CStr {
        (*self).into()
    }
}

impl Deref for ThinCStr<'_> {
    type Target = CStr;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl fmt::Debug for ThinCStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.deref(), f)
    }
}

impl PartialEq for ThinCStr<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.deref() == other.deref()
    }
}

impl PartialEq<&CStr> for ThinCStr<'_> {
    fn eq(&self, other: &&CStr) -> bool {
        self.deref() == *other
    }
}

impl PartialEq<&str> for ThinCStr<'_> {
    fn eq(&self, other: &&str) -> bool {
        self.deref().to_bytes() == other.as_bytes()
    }
}

impl Eq for ThinCStr<'_> {}

/// A borrowed, nul-terminated wide (u16) string of unspecified encoding with the same ABI as
/// [`NonNull`].
///
/// The [`Debug`] implementation for this type interprets the string as potentially invalid UTF-16.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct ThinWCStr<'a>(NonNull<u16>, PhantomData<&'a ()>);

impl ThinWCStr<'_> {
    /// Construct a [`ThinWStr`] from a pointer to a nul-terminated wide C string.
    ///
    /// # Safety
    ///
    /// Has the same invariants as [`CStr::from_ptr`], except that the element type is a `u16`
    /// instead of a `c_char`.
    pub unsafe fn from_ptr(ptr: NonNull<u16>) -> Self {
        Self(ptr, PhantomData)
    }

    /// Get a raw pointer to the beginning of this wide C string.
    pub fn as_ptr(&self) -> *const u16 {
        self.0.as_ptr()
    }

    /// Get the length of this wide C string in two-byte units, excluding the nul terminator.
    pub fn len(&self) -> usize {
        // SAFETY: By the invariants of the type
        unsafe {
            let mut len = 0;
            while self.0.add(len).read() != 0 {
                len += 1;
            }
            len
        }
    }

    /// Check if this C string is empty.
    pub fn is_empty(&self) -> bool {
        // SAFETY: By the invariants of the type
        unsafe { self.0.read() == 0 }
    }

    /// Converts this C wstring to a u16 slice.
    ///
    /// The returned slice will not contain the trailing nul terminator.
    pub fn to_wchars(&self) -> &[u16] {
        // SAFETY: By the invariants of the type
        unsafe { std::slice::from_raw_parts(self.0.as_ptr(), self.len()) }
    }

    /// Converts this C wstring to a u16 slice, including the nul terminator.
    pub fn to_wchars_with_nul(&self) -> &[u16] {
        // SAFETY: By the invariants of the type
        unsafe { std::slice::from_raw_parts(self.0.as_ptr(), self.len() + 1) }
    }

    /// Attempt to convert this string to a Rust string, assuming UTF-16 encoding.
    pub fn to_string(&self) -> Result<String, DecodeUtf16Error> {
        char::decode_utf16(self.to_wchars().iter().copied()).collect()
    }

    /// Convert this string to a Rust string, assuming UTF-16 encoding.
    ///
    /// Invalid code points are replaced by [`char::REPLACEMENT_CHARACTER`].
    pub fn to_string_lossy(&self) -> String {
        char::decode_utf16(self.to_wchars().iter().copied())
            .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect()
    }
}

impl TryFrom<ThinWCStr<'_>> for String {
    type Error = DecodeUtf16Error;

    fn try_from(value: ThinWCStr<'_>) -> Result<Self, Self::Error> {
        value.to_string()
    }
}

impl AsRef<[u16]> for ThinWCStr<'_> {
    fn as_ref(&self) -> &[u16] {
        self.to_wchars()
    }
}

impl fmt::Debug for ThinWCStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use fmt::Write;

        let mut char_iter = char::decode_utf16(self.to_wchars().iter().copied())
            .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER));

        // A Debug representation for a string should be escaped. Instead of
        // reimplementing the escaping logic, we convert to utf-8 first so the
        // Debug impl of str can be used.
        let mut stack_str = [0u8; 128];
        let mut stack_str_len = 0;
        let mut heap_str;
        let lossy_str = loop {
            if let Some(c) = char_iter.next() {
                // Possibly not enough space to encode the next character,
                // fall back to a heap-allocated string.
                if stack_str_len > stack_str.len() - 4 {
                    heap_str =
                        unsafe { String::from_utf8_unchecked(stack_str[..stack_str_len].into()) };
                    heap_str.write_char(c)?;
                    for c in char_iter {
                        heap_str.write_char(c)?;
                    }
                    break heap_str.as_str();
                }
                stack_str_len += c.encode_utf8(&mut stack_str[stack_str_len..]).len();
            } else {
                break unsafe { str::from_utf8_unchecked(&stack_str[..stack_str_len]) };
            }
        };

        std::fmt::Debug::fmt(lossy_str, f)
    }
}

impl PartialEq for ThinWCStr<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.to_wchars() == other.to_wchars()
    }
}

impl PartialEq<&[u16]> for ThinWCStr<'_> {
    fn eq(&self, other: &&[u16]) -> bool {
        self.to_wchars() == *other
    }
}

impl Eq for ThinWCStr<'_> {}
