use std::{
    collections::{HashMap, VecDeque},
    env,
    ffi::OsStr,
    fmt,
    fs::read_dir,
    iter,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf, StripPrefixError},
};

use me3_mod_protocol::package::AssetOverrideSource;
use normpath::PathExt;
use thiserror::Error;

pub struct ArchiveOverrideMapping {
    map: HashMap<PathBuf, (String, Vec<u16>)>,
    current_dir: PathBuf,
}

#[derive(Debug, Error)]
pub enum ArchiveOverrideMappingError {
    #[error("Package source specified is not a directory {0}.")]
    InvalidDirectory(PathBuf),

    #[error("Could not read directory while discovering override assets {0}")]
    ReadDir(std::io::Error),

    #[error("Could not acquire directory entry")]
    DirEntryAcquire(std::io::Error),

    #[error("Could not acquire directory entry")]
    StripPrefix(#[from] StripPrefixError),
}

impl ArchiveOverrideMapping {
    pub fn new() -> Result<Self, ArchiveOverrideMappingError> {
        let current_dir = env::current_dir().map_err(ArchiveOverrideMappingError::ReadDir)?;

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

        let mut paths_to_scan = VecDeque::from(vec![base_directory.clone()]);
        while let Some(current_path) = paths_to_scan.pop_front() {
            let Ok(entries) = read_dir(&current_path) else {
                tracing::error!(path = ?current_path, "unable to read asset override files in directory");
                continue;
            };

            for dir_entry in entries {
                let dir_entry = dir_entry
                    .map_err(ArchiveOverrideMappingError::DirEntryAcquire)?
                    .path();

                if dir_entry.is_dir() {
                    paths_to_scan.push_back(dir_entry);
                } else {
                    let override_path = dir_entry
                        .normalize_virtually()
                        .map_err(ArchiveOverrideMappingError::DirEntryAcquire)?;

                    let vfs_path = path_to_asset_lookup_key(&base_directory, &override_path)?;
                    let as_wide = override_path.encode_wide_with_nul().collect();

                    self.map.insert(
                        vfs_path,
                        (override_path.as_os_str().display().to_string(), as_wide),
                    );
                }
            }
        }

        Ok(())
    }

    pub fn get_override<S: AsRef<OsStr>>(&self, path_str: S) -> Option<(&str, &[u16])> {
        let from_map = |k: &Path| self.map.get(k).map(|(p, w)| (&**p, &**w));

        if let Ok(norm) = Path::new(&path_str).normalize() {
            if let Ok(key) = norm.as_path().strip_prefix(&self.current_dir) {
                return from_map(key);
            }
        }

        from_map(Path::new(split_virtual_root(&path_str)))
    }
}

/// Turns an asset path into an asset lookup key using the mod's base path.
fn path_to_asset_lookup_key<P1: AsRef<Path>, P2: AsRef<Path>>(
    base: P1,
    path: P2,
) -> Result<PathBuf, StripPrefixError> {
    path.as_ref()
        .strip_prefix(base)
        .map(|p| p.as_os_str().to_string_lossy().to_lowercase().into())
}

/// Returns the path without its virtual root.
fn split_virtual_root<S: AsRef<OsStr>>(s: &S) -> &OsStr {
    let bytes = s.as_ref().as_encoded_bytes();

    // SAFETY: splitting after a valid ASCII character.
    bytes
        .windows(2)
        .position(|w| w[0] == b':' && (w[1] == b'/' || w[1] == b'\\'))
        .map_or(s.as_ref(), |i| unsafe {
            OsStr::from_encoded_bytes_unchecked(&bytes[i + 2..])
        })
}

impl fmt::Debug for ArchiveOverrideMapping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(self.map.iter().map(|(k, (v, _))| (k, v)))
            .finish()
    }
}

trait OsStrEncodeExt {
    fn encode_wide_with_nul(&self) -> impl Iterator<Item = u16>;
}

impl<T: AsRef<OsStr>> OsStrEncodeExt for T {
    fn encode_wide_with_nul(&self) -> impl Iterator<Item = u16> {
        self.as_ref().encode_wide().chain(iter::once(0))
    }
}

#[cfg(test)]
mod test {
    use std::path::{Path, PathBuf};

    use crate::mapping::{path_to_asset_lookup_key, ArchiveOverrideMapping};

    #[test]
    fn asset_path_lookup_keys() {
        const FAKE_MOD_BASE: &str = "D:/ModBase/";
        let base_path = PathBuf::from(FAKE_MOD_BASE);

        assert_eq!(
            path_to_asset_lookup_key(
                &base_path,
                PathBuf::from(format!(
                    "{FAKE_MOD_BASE}/parts/aet/aet007/aet007_071.tpf.dcx"
                )),
            )
            .unwrap(),
            Path::new("parts/aet/aet007/aet007_071.tpf.dcx"),
        );

        assert_eq!(
            path_to_asset_lookup_key(
                &base_path,
                PathBuf::from(format!(
                    "{FAKE_MOD_BASE}/hkxbnd/m60_42_36_00/h60_42_36_00_423601.hkx.dcx"
                )),
            )
            .unwrap(),
            Path::new("hkxbnd/m60_42_36_00/h60_42_36_00_423601.hkx.dcx"),
        );

        assert_eq!(
            path_to_asset_lookup_key(
                &base_path,
                PathBuf::from(format!("{FAKE_MOD_BASE}/regulation.bin")),
            )
            .unwrap(),
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
                .get_override("data0:/regulation.bin")
                .is_some(),
            "override for regulation.bin was not found"
        );
        assert!(
            asset_mapping
                .get_override("data0:/event/common.emevd.dcx")
                .is_some(),
            "override for event/common.emevd.dcx not found"
        );
        assert!(
            asset_mapping
                .get_override("data0:/common.emevd.dcx")
                .is_none(),
            "event/common.emevd.dcx was found incorrectly under the regulation root"
        );
    }
}
