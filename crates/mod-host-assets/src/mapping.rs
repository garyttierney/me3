use std::{
    borrow::Borrow,
    collections::{HashMap, VecDeque},
    env,
    ffi::OsStr,
    fmt,
    fs::read_dir,
    io,
    os::windows::ffi::OsStrExt as WinOsStrExt,
    path::{Path, PathBuf, StripPrefixError},
};

use me3_mod_protocol::package::AssetOverrideSource;
use normpath::PathExt;
use thiserror::Error;
use tracing::error;
use windows::core::{PCSTR, PCWSTR};

#[derive(Debug)]
pub struct ArchiveOverrideMapping {
    map: HashMap<VfsKey, ArchiveOverride>,
    current_dir: VfsKey,
}

pub struct ArchiveOverride {
    display: Box<str>,
    path_c_str: Box<Path>,
    wide_c_str: Box<[u16]>,
}

#[derive(Debug, Error)]
pub enum ArchiveOverrideMappingError {
    #[error("Package source specified is not a directory {0}.")]
    InvalidDirectory(PathBuf),

    #[error("Could not read directory while discovering override assets {0}")]
    ReadDir(io::Error),

    #[error("Could not acquire directory entry")]
    StripPrefix(#[from] StripPrefixError),
}

impl ArchiveOverrideMapping {
    pub fn new() -> Result<Self, ArchiveOverrideMappingError> {
        let current_dir = env::current_dir()
            .and_then(VfsKey::for_disk_path)
            .map_err(ArchiveOverrideMappingError::ReadDir)?;

        Ok(Self {
            map: HashMap::new(),
            current_dir,
        })
    }

    /// Scans a set of directories, mapping discovered assets into itself.
    pub fn scan_directories<I, S>(&mut self, sources: I) -> Result<(), ArchiveOverrideMappingError>
    where
        I: Iterator<Item = S>,
        S: AssetOverrideSource,
    {
        sources
            .map(|p| self.scan_directory(p.asset_path()))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(())
    }

    /// Traverses a folder structure, mapping discovered assets into itself.
    pub fn scan_directory<P: AsRef<Path>>(
        &mut self,
        base_directory: P,
    ) -> Result<(), ArchiveOverrideMappingError> {
        let base_directory = base_directory
            .as_ref()
            .to_path_buf()
            .normalize()
            .map_err(ArchiveOverrideMappingError::ReadDir)?
            .into_path_buf();

        if !base_directory.is_dir() {
            return Err(ArchiveOverrideMappingError::InvalidDirectory(
                base_directory.to_path_buf(),
            ));
        }

        let base_directory_key =
            VfsKey::for_disk_path(&base_directory).map_err(ArchiveOverrideMappingError::ReadDir)?;

        let mut paths_to_scan = VecDeque::from(vec![base_directory.clone()]);
        while let Some(current_path) = paths_to_scan.pop_front() {
            let Ok(entries) = read_dir(&current_path) else {
                error!(path = ?current_path, "unable to read asset override files in directory");
                continue;
            };

            for dir_entry in entries.flatten().map(|e| e.path()) {
                if dir_entry.is_dir() {
                    paths_to_scan.push_back(dir_entry);
                    continue;
                }

                let Ok(vfs_key) = VfsKey::for_asset_path(&dir_entry, &base_directory_key) else {
                    continue;
                };

                let archive_override = ArchiveOverride::new(dir_entry);

                self.map.insert(vfs_key, archive_override);
            }
        }

        Ok(())
    }

    pub fn vfs_override<S: AsRef<OsStr>>(&self, path_str: S) -> Option<&ArchiveOverride> {
        let key = VfsKey::for_vfs_path(Path::new(&path_str));
        self.map.get(&key)
    }

    pub fn disk_override<S: AsRef<OsStr>>(&self, path_str: S) -> Option<&ArchiveOverride> {
        let key = VfsKey::for_asset_path(Path::new(&path_str), &self.current_dir).ok()?;
        self.map.get(&key)
    }
}

impl ArchiveOverride {
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
        // which is always inserted by `ArchiveOverride::new`
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

impl fmt::Debug for ArchiveOverride {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArchiveOverride")
            .field("display", &self.display)
            .field("path", &self.as_path())
            .finish()
    }
}

impl fmt::Display for ArchiveOverride {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.display.fmt(f)
    }
}

impl AsRef<Path> for ArchiveOverride {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl AsRef<[u16]> for ArchiveOverride {
    fn as_ref(&self) -> &[u16] {
        self.as_wide()
    }
}

impl From<&ArchiveOverride> for PCSTR {
    fn from(value: &ArchiveOverride) -> Self {
        value.as_pcstr()
    }
}

impl From<&ArchiveOverride> for PCWSTR {
    fn from(value: &ArchiveOverride) -> Self {
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
        let Self(normalized) = Self::for_disk_path(path)?;

        let stripped = normalized
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

    use super::{ArchiveOverrideMapping, VfsKey};

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
        let mut asset_mapping = ArchiveOverrideMapping::new().unwrap();

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
