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
    /// Name associated with this item.
    pub name: String,

    /// A path to the source of this item.
    pub path: PathBuf,

    /// Does this item participate in dependency resolution?
    pub enabled: bool,

    /// Should failing to find this item result in a hard error?
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
}

impl Default for ModFile {
    #[inline]
    fn default() -> Self {
        Self {
            name: Default::default(),
            path: Default::default(),
            enabled: true,
            optional: false,
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
