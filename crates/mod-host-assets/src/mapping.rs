use std::{
    collections::{HashMap, VecDeque},
    fs::read_dir,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf, StripPrefixError},
};

use me3_mod_protocol::package::AssetOverrideSource;
use thiserror::Error;

#[derive(Debug, Default)]
pub struct ArchiveOverrideMapping {
    map: HashMap<String, (PathBuf, Vec<u16>)>,
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

    ///  Traverses a folder structure, mapping discovered assets into itself.
    pub fn scan_directory<P: AsRef<Path>>(
        &mut self,
        base_directory: P,
    ) -> Result<(), ArchiveOverrideMappingError> {
        let base_directory = normalize_path(base_directory.as_ref());
        if !base_directory.is_dir() {
            return Err(ArchiveOverrideMappingError::InvalidDirectory(
                base_directory.clone(),
            ));
        }

        let mut paths_to_scan = VecDeque::from(vec![base_directory.clone()]);
        while let Some(current_path) = paths_to_scan.pop_front() {
            for dir_entry in read_dir(current_path).map_err(ArchiveOverrideMappingError::ReadDir)? {
                let dir_entry = dir_entry
                    .map_err(ArchiveOverrideMappingError::DirEntryAcquire)?
                    .path();

                if dir_entry.is_dir() {
                    paths_to_scan.push_back(dir_entry);
                } else {
                    let override_path = normalize_path(dir_entry);
                    let vfs_path = path_to_asset_lookup_key(&base_directory, &override_path)?;
                    let as_wide = override_path.as_os_str().encode_wide().collect();

                    self.map.insert(vfs_path, (override_path, as_wide));
                }
            }
        }

        Ok(())
    }

    pub fn get_override(&self, path: &str) -> Option<(&Path, &[u16])> {
        let key = path.split_once(":/").map(|r| r.1).unwrap_or(path);

        self.map.get(key).map(|(path, wide)| (&**path, &**wide))
    }
}

/// Normalizes paths to use / as a path seperator.
fn normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    PathBuf::from(path.as_ref().to_string_lossy().replace('\\', "/"))
}

/// Turns an asset path into an asset lookup key using the mods base path.
fn path_to_asset_lookup_key<P1: AsRef<Path>, P2: AsRef<Path>>(
    base: P1,
    path: P2,
) -> Result<String, StripPrefixError> {
    path.as_ref()
        .strip_prefix(base)
        .map(|p| p.to_string_lossy().to_lowercase())
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
                &PathBuf::from(format!(
                    "{FAKE_MOD_BASE}/parts/aet/aet007/aet007_071.tpf.dcx"
                )),
            )
            .unwrap(),
            "parts/aet/aet007/aet007_071.tpf.dcx",
        );

        assert_eq!(
            path_to_asset_lookup_key(
                &base_path,
                &PathBuf::from(format!(
                    "{FAKE_MOD_BASE}/hkxbnd/m60_42_36_00/h60_42_36_00_423601.hkx.dcx"
                )),
            )
            .unwrap(),
            "hkxbnd/m60_42_36_00/h60_42_36_00_423601.hkx.dcx",
        );

        assert_eq!(
            path_to_asset_lookup_key(
                &base_path,
                &PathBuf::from(format!("{FAKE_MOD_BASE}/regulation.bin")),
            )
            .unwrap(),
            "regulation.bin",
        );
    }

    #[test]
    fn scan_directory_and_overrides() {
        let mut asset_mapping = ArchiveOverrideMapping::default();

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
