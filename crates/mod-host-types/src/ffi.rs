//! Generic FFI-related types that are not specific to FromSoftware games.

pub mod thin_cstr;

#[doc(inline)]
pub use thin_cstr::{ThinCStr, ThinWCStr};
