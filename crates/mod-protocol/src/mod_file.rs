use std::{
    ops::BitXor,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

pub trait AsModFile {
    fn as_mod_file(&self) -> &ModFile;
    fn as_mod_file_mut(&mut self) -> &mut ModFile;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ModFile {
    /// Name associated with this file.
    pub name: String,

    /// A path to the source of this file.
    pub path: PathBuf,

    /// Does this file participate in dependency resolution?
    #[serde(
        default = "ModFile::enabled_default",
        skip_serializing_if = "ModFile::enabled_is_default"
    )]
    pub enabled: bool,

    /// Should failing to find this file result in a hard error?
    #[serde(
        default = "ModFile::optional_default",
        skip_serializing_if = "ModFile::optional_is_default"
    )]
    pub optional: bool,
}

impl ModFile {
    #[inline]
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        path.as_ref().to_owned().into()
    }

    #[inline]
    pub fn is_relative(&self) -> bool {
        self.path.is_relative()
    }

    #[inline]
    pub fn is_default(&self) -> bool {
        self.enabled && !self.optional
    }

    #[inline]
    pub fn make_absolute<P: AsRef<Path>>(&mut self, base: P) {
        if self.path.is_relative() {
            self.path = base.as_ref().join(&self.path);
        }
    }

    #[inline]
    pub(crate) fn enabled_default() -> bool {
        true
    }

    #[inline]
    pub(crate) fn enabled_is_default(enabled: &bool) -> bool {
        *enabled == Self::enabled_default()
    }

    #[inline]
    pub(crate) fn optional_default() -> bool {
        false
    }

    #[inline]
    pub(crate) fn optional_is_default(optional: &bool) -> bool {
        *optional == Self::optional_default()
    }
}

impl Default for ModFile {
    #[inline]
    fn default() -> Self {
        Self {
            name: Default::default(),
            path: Default::default(),
            enabled: Self::enabled_default(),
            optional: Self::optional_default(),
        }
    }
}

impl AsRef<Path> for ModFile {
    #[inline]
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl From<PathBuf> for ModFile {
    #[inline]
    fn from(path: PathBuf) -> Self {
        let fnv1_a = |b: &[u8]| {
            b.iter().fold(0x811c9dc5u32, |hash, byte| {
                hash.bitxor(*byte as u32).wrapping_mul(0x01000193)
            })
        };

        Self {
            name: format!(
                "{}_{:x}",
                path.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase(),
                fnv1_a(path.as_os_str().as_encoded_bytes())
            ),
            path,
            ..Default::default()
        }
    }
}

impl AsModFile for ModFile {
    #[inline]
    fn as_mod_file(&self) -> &ModFile {
        self
    }

    #[inline]
    fn as_mod_file_mut(&mut self) -> &mut ModFile {
        self
    }
}
