use std::{
    borrow::Borrow,
    collections::HashMap,
    env,
    ffi::OsStr,
    fmt,
    fs::read_dir,
    io, iter,
    os::windows::{ffi::OsStrExt as WinOsStrExt, fs::FileTypeExt},
    path::{Path, PathBuf, StripPrefixError},
};

use me3_mod_protocol::package::{AssetOverrideSource, Package};
use normpath::PathExt;
use rayon::iter::{ParallelBridge, ParallelIterator};
use smallvec::{smallvec_inline, SmallVec};
use thiserror::Error;
use windows::core::{PCSTR, PCWSTR};

use crate::platform::normalize_dos_path;

mod savefile;

pub struct VfsOverrideMapping {
    map: HashMap<VfsKey, VfsOverride>,
    current_dir: VfsKey,
    savefile_override: Option<savefile::SavefileOverrideMapping>,
}

pub struct VfsOverride {
    display: Box<str>,
    path_c_str: Box<Path>,
    wide_c_str: Box<[u16]>,
}

#[derive(Debug, Error)]
pub enum VfsOverrideMappingError {
    #[error("An error occurred while converting Linux paths for WINE")]
    Compatibility,

    #[error("Package source specified is not a directory {0}.")]
    InvalidDirectory(PathBuf),

    #[error("Could not read directory while discovering override assets {0}")]
    ReadDir(io::Error),

    #[error("Could not acquire directory entry")]
    StripPrefix(#[from] StripPrefixError),
}

impl VfsOverrideMapping {
    pub fn new() -> Result<Self, VfsOverrideMappingError> {
        let current_dir = env::current_dir()
            .and_then(VfsKey::for_disk_path)
            .map_err(VfsOverrideMappingError::ReadDir)?;

        Ok(Self {
            map: HashMap::new(),
            current_dir,
            savefile_override: None,
        })
    }

    /// Scans a set of directories, mapping discovered assets into itself.
    pub fn scan_directories<I>(&mut self, sources: I) -> Result<(), VfsOverrideMappingError>
    where
        I: Iterator<Item: AssetOverrideSource>,
    {
        fn scan_directories_inner(
            base_dir: &Path,
            root_key: &VfsKey,
        ) -> SmallVec<[Result<(VfsKey, VfsOverride), io::Error>; 1]> {
            let entries = match read_dir(base_dir) {
                Ok(entries) => entries,
                Err(e) => return smallvec_inline![Err(e)],
            };

            let result = entries
                .flatten()
                .par_bridge()
                .flat_map_iter(|dir_entry| match dir_entry.file_type() {
                    Ok(file_type) if file_type.is_dir() || file_type.is_symlink_dir() => {
                        scan_directories_inner(&dir_entry.path(), root_key)
                    }
                    Ok(_) => {
                        let path = dir_entry.path();

                        let result = VfsKey::for_asset_path(&path, root_key)
                            .map(|vfs_key| (vfs_key, VfsOverride::new(&path)));

                        smallvec_inline![result]
                    }
                    Err(e) => smallvec_inline![Err(e)],
                })
                .collect();

            SmallVec::from_vec(result)
        }

        for source in sources {
            let source_path = source.asset_path();
            let normalized_path = normalize_dos_path(source_path)?;
            let root_key = VfsKey::for_disk_path(&normalized_path)
                .map_err(VfsOverrideMappingError::ReadDir)?;

            let scanned_directories = scan_directories_inner(&normalized_path, &root_key);
            self.map.reserve(scanned_directories.len());

            for result in scanned_directories {
                let (vfs_key, vfs_override) = result.map_err(VfsOverrideMappingError::ReadDir)?;
                self.map.insert(vfs_key, vfs_override);
            }
        }

        Ok(())
    }

    pub fn scan_directory<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<(), VfsOverrideMappingError> {
        let package = Package::new(path.as_ref().to_owned());
        self.scan_directories(iter::once(&package))
    }

    pub fn add_savefile_override<P, F>(&mut self, savefile_dir: P, f: F) -> Result<(), io::Error>
    where
        P: AsRef<Path>,
        F: Fn(&Path) -> PathBuf + Send + Sync + 'static,
    {
        let savefile_override = savefile::SavefileOverrideMapping::new(savefile_dir, f)?;
        self.savefile_override = Some(savefile_override);
        Ok(())
    }

    pub fn vfs_override<S: AsRef<OsStr>>(&self, path_str: S) -> Option<&VfsOverride> {
        let path = Path::new(&path_str);

        if let Some(savefile_override) = &self.savefile_override
            && let Ok(key) = VfsKey::for_disk_path(path)
            && let Some(savefile_override_path) = savefile_override.try_override(path, &key)
        {
            return Some(savefile_override_path);
        }

        let key = VfsKey::for_vfs_path(path);
        self.map.get(&key)
    }

    pub fn disk_override<S: AsRef<OsStr>>(&self, path_str: S) -> Option<&VfsOverride> {
        let key = VfsKey::for_asset_path(Path::new(&path_str), &self.current_dir).ok()?;
        self.map.get(&key)
    }
}

impl VfsOverride {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let display = path.as_ref().display().to_string().into_boxed_str();

        let (wide_c_str, path_c_str) = {
            let mut os_str = path.as_ref().as_os_str().to_os_string();
            os_str.push("\0");

            (
                Vec::into_boxed_slice(os_str.encode_wide().collect()),
                PathBuf::into_boxed_path(os_str.into()),
            )
        };

        Self {
            display,
            path_c_str,
            wide_c_str,
        }
    }

    pub fn as_str_lossy(&self) -> &str {
        &self.display
    }

    pub fn as_path(&self) -> &Path {
        let bytes_with_nul = self.path_c_str.as_os_str().as_encoded_bytes();
        let bytes_without_nul = &bytes_with_nul[..bytes_with_nul.len() - 1];

        // SAFETY: Source OsStr bytes split before valid substring ("\0"),
        // which is always inserted by `VfsOverride::new`
        unsafe { Path::new(OsStr::from_encoded_bytes_unchecked(bytes_without_nul)) }
    }

    pub fn as_wide(&self) -> &[u16] {
        &self.wide_c_str[..self.wide_c_str.len() - 1]
    }

    pub fn as_c_str(&self) -> *const u8 {
        self.path_c_str.as_os_str().as_encoded_bytes().as_ptr()
    }

    pub fn as_wide_c_str(&self) -> *const u16 {
        self.wide_c_str.as_ptr()
    }

    pub fn as_pcstr(&self) -> PCSTR {
        PCSTR::from_raw(self.as_c_str())
    }

    pub fn as_pcwstr(&self) -> PCWSTR {
        PCWSTR::from_raw(self.as_wide_c_str())
    }
}

impl fmt::Debug for VfsOverride {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VfsOverride")
            .field("display", &self.display)
            .field("path", &self.as_path())
            .finish()
    }
}

impl fmt::Display for VfsOverride {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.display.fmt(f)
    }
}

impl AsRef<Path> for VfsOverride {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl AsRef<[u16]> for VfsOverride {
    fn as_ref(&self) -> &[u16] {
        self.as_wide()
    }
}

impl From<&VfsOverride> for PCSTR {
    fn from(value: &VfsOverride) -> Self {
        value.as_pcstr()
    }
}

impl From<&VfsOverride> for PCWSTR {
    fn from(value: &VfsOverride) -> Self {
        value.as_pcwstr()
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct VfsKey(Box<Path>);

impl VfsKey {
    /// Turns a disk path into an asset lookup key that includes the root directory.
    fn for_disk_path<P: AsRef<Path>>(path: P) -> Result<Self, io::Error> {
        let normalized = path
            .as_ref()
            .normalize_virtually()?
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
            .collect();

        Ok(Self(PathBuf::into_boxed_path(normalized)))
    }

    /// Turns a vfs path into an asset lookup key that does not include the root directory.
    fn for_vfs_path<P: AsRef<Path>>(path: P) -> Self {
        let normalized = path
            .as_ref()
            .components()
            .skip_while(|c| matches!(c.as_os_str().as_encoded_bytes().last(), Some(b':')))
            .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
            .collect();

        Self(PathBuf::into_boxed_path(normalized))
    }

    /// Turns a disk path into an asset lookup key that does not include the root directory.
    fn for_asset_path<P: AsRef<Path>>(path: P, base: &Self) -> Result<Self, io::Error> {
        Self::for_disk_path(path)?.strip_prefix(base)
    }

    /// Strips the root directory from a disk asset lookup key.
    fn strip_prefix(&self, base: &Self) -> Result<Self, io::Error> {
        let stripped = self
            .0
            .strip_prefix(base)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidFilename, e))?;

        Ok(Self(stripped.into()))
    }
}

impl AsRef<Path> for VfsKey {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Borrow<Path> for VfsKey {
    fn borrow(&self) -> &Path {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use super::{VfsKey, VfsOverrideMapping};

    #[test]
    fn asset_path_lookup_keys() {
        const FAKE_MOD_BASE: &str = "D:/ModBase";
        let base_path = VfsKey::for_disk_path(Path::new(FAKE_MOD_BASE)).unwrap();

        assert_eq!(
            VfsKey::for_asset_path(
                Path::new(&format!(
                    "{FAKE_MOD_BASE}/parts/aet/aet007/aet007_071.tpf.dcx"
                )),
                &base_path
            )
            .unwrap()
            .as_ref(),
            Path::new("parts/aet/aet007/aet007_071.tpf.dcx"),
        );

        assert_eq!(
            VfsKey::for_asset_path(
                Path::new(&format!(
                    "{FAKE_MOD_BASE}/hkxbnd/m60_42_36_00/h60_42_36_00_423601.hkx.dcx"
                )),
                &base_path
            )
            .unwrap()
            .as_ref(),
            Path::new("hkxbnd/m60_42_36_00/h60_42_36_00_423601.hkx.dcx"),
        );

        assert_eq!(
            VfsKey::for_asset_path(
                Path::new(&format!("{FAKE_MOD_BASE}/regulation.bin")),
                &base_path
            )
            .unwrap()
            .as_ref(),
            Path::new("regulation.bin"),
        );
    }

    #[test]
    fn scan_directory_and_overrides() {
        let mut asset_mapping = VfsOverrideMapping::new().unwrap();

        let test_mod_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data/test-mod");
        asset_mapping.scan_directory(test_mod_dir).unwrap();

        assert!(
            asset_mapping
                .vfs_override("data0:/regulation.bin")
                .is_some(),
            "override for regulation.bin was not found"
        );
        assert!(
            asset_mapping
                .vfs_override("data0:/event/common.emevd.dcx")
                .is_some(),
            "override for event/common.emevd.dcx not found"
        );
        assert!(
            asset_mapping
                .vfs_override("data0:/common.emevd.dcx")
                .is_none(),
            "event/common.emevd.dcx was found incorrectly under the regulation root"
        );
    }
}
