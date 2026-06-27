use std::{
    borrow::{Borrow, Cow},
    collections::HashMap,
    env,
    ffi::{OsStr, OsString},
    fmt,
    fs::read_dir,
    io, iter,
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        fs::FileTypeExt,
    },
    path::{Path, PathBuf, StripPrefixError},
};

use me3_mod_protocol::package::{AssetOverrideSource, Package};
use normpath::PathExt;
use rayon::iter::{ParallelBridge, ParallelIterator};
use slab::Slab;
use smallvec::{smallvec_inline, SmallVec};
use thiserror::Error;
use windows::core::PCWSTR;
use xxhash_rust::xxh3::Xxh3DefaultBuilder;

use crate::{mapping::savefile::SavefileOverrideMapping, platform::normalize_dos_path};

mod savefile;

pub struct VfsOverrideMapping {
    current_dir: VfsKey,
    vfs_map: HashMap<VfsKey, usize, Xxh3DefaultBuilder>,
    overrides: Slab<VfsOverride<'static>>,
    savefile_override: Option<SavefileOverrideMapping>,
}

#[derive(Clone)]
pub struct VfsOverride<'a> {
    generation: Generation,
    wide_c_str: Box<[u16]>,
    display: Cow<'a, str>,
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
            current_dir,
            vfs_map: HashMap::default(),
            overrides: Slab::new(),
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
        ) -> SmallVec<[Result<(VfsKey, VfsOverride<'static>), io::Error>; 1]> {
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

                        let result = VfsKey::for_asset_path(&path, root_key).map(|vfs_key| {
                            let display = path.to_string_lossy().into_owned();
                            (vfs_key, VfsOverride::new(path, Generation, display.into()))
                        });

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

            self.overrides.reserve(scanned_directories.len());
            self.vfs_map.reserve(scanned_directories.len());

            for result in scanned_directories {
                let (vfs_key, vfs_override) = result.map_err(VfsOverrideMappingError::ReadDir)?;

                let index = self.overrides.insert(vfs_override);
                self.vfs_map.insert(vfs_key, index);
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
        let savefile_override = SavefileOverrideMapping::new(savefile_dir, f)?;
        self.savefile_override = Some(savefile_override);
        Ok(())
    }

    pub fn virtual_to_disk<S: AsRef<OsStr>>(&self, path_str: S) -> Option<&VfsOverride<'static>> {
        let path = Path::new(&path_str);

        if let Some(savefile_override) = self.virtual_to_savefile(path) {
            return Some(savefile_override);
        }

        let key = VfsKey::for_vfs_path(path);
        let index = *self.vfs_map.get(&key)?;

        self.overrides.get(index)
    }

    pub fn virtual_to_uid<S: AsRef<OsStr>>(&self, path_str: S) -> Option<VfsOverride<'_>> {
        let path = Path::new(&path_str);

        if let Some(savefile_override) = self.virtual_to_savefile(path) {
            return Some(savefile_override.clone());
        }

        let key = VfsKey::for_vfs_path(path);
        let index = *self.vfs_map.get(&key)?;

        let vfs_override = self.overrides.get(index)?;
        let vfs_uid = VfsUid::new(index, vfs_override.generation);

        let uid_path = vfs_uid.to_uid_string();

        Some(VfsOverride::new(
            uid_path,
            vfs_override.generation,
            Cow::Borrowed(&vfs_override.display),
        ))
    }

    pub fn disk_or_uid_to_disk<S: AsRef<OsStr>>(
        &self,
        path_str: S,
    ) -> Option<&VfsOverride<'static>> {
        if let Some(from_uid) = self.uid_to_disk(path_str.as_ref()) {
            return Some(from_uid);
        }

        let key = VfsKey::for_asset_path(Path::new(&path_str), &self.current_dir).ok()?;
        let index = self.vfs_map.get(&key)?;

        self.overrides.get(*index)
    }

    fn virtual_to_savefile(&self, path: &Path) -> Option<&VfsOverride<'static>> {
        let savefile_override = self.savefile_override.as_ref()?;
        let key = VfsKey::for_disk_path(path).ok()?;
        savefile_override.try_override(path, &key)
    }

    fn uid_to_disk(&self, uid_str: &OsStr) -> Option<&VfsOverride<'static>> {
        let VfsUid { generation, index } = VfsUid::try_parse(uid_str)?;
        let vfs_override = self.overrides.get(index)?;

        (generation == vfs_override.generation).then_some(vfs_override)
    }
}

impl<'a> VfsOverride<'a> {
    fn new<P: AsRef<OsStr>>(path: P, generation: Generation, display: Cow<'a, str>) -> Self {
        Self {
            generation,
            wide_c_str: path.as_ref().encode_wide().chain([0]).collect(),
            display,
        }
    }

    pub fn to_path_buf(&self) -> PathBuf {
        PathBuf::from(OsString::from_wide(self.as_wide()))
    }

    pub fn to_c_string(&self) -> Vec<u8> {
        OsString::from_wide(self.as_wide_c_string()).into_encoded_bytes()
    }

    pub fn as_wide(&self) -> &[u16] {
        &self.wide_c_str[..self.wide_c_str.len() - 1]
    }

    pub fn as_wide_c_string(&self) -> &[u16] {
        &self.wide_c_str
    }

    pub fn as_pcwstr(&self) -> PCWSTR {
        PCWSTR(self.as_wide_c_string().as_ptr())
    }

    pub fn as_display_str(&self) -> &str {
        &self.display
    }
}

impl AsRef<[u16]> for VfsOverride<'_> {
    fn as_ref(&self) -> &[u16] {
        self.as_wide()
    }
}

impl From<&VfsOverride<'_>> for PCWSTR {
    fn from(vfs_override: &VfsOverride<'_>) -> Self {
        vfs_override.as_pcwstr()
    }
}

impl fmt::Debug for VfsOverride<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VfsOverride")
            .field("generation", &self.generation)
            .field("path", &self.to_path_buf())
            .field("display", &self.as_display_str())
            .finish()
    }
}

impl fmt::Display for VfsOverride<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_display_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct VfsUid {
    generation: Generation,
    index: usize,
}

// May become a `usize` in the future to implement asset reloading.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Generation;

impl VfsUid {
    const ROOT: &str = r"\\me3";

    fn new(index: usize, generation: Generation) -> Self {
        Self { generation, index }
    }

    pub fn to_uid_string(self) -> String {
        self.with_fmt_args(|fmt| format!("{fmt}"))
    }

    pub fn try_parse(str: &OsStr) -> Option<Self> {
        let str = str.to_str()?;

        let index_str = str.strip_prefix(Self::ROOT)?.strip_prefix("??")?;
        let index = usize::from_str_radix(index_str, 16).ok()?;

        Some(Self::new(index, Generation))
    }

    #[inline(always)]
    fn with_fmt_args<T>(&self, f: impl FnOnce(fmt::Arguments<'_>) -> T) -> T {
        let root = Self::ROOT;
        let generation = "";
        let index = self.index;

        f(format_args!("{root}?{generation}?{index:x}"))
    }
}

impl fmt::Display for VfsUid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.with_fmt_args(|fmt| f.write_fmt(fmt))
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
                .virtual_to_uid("data0:/regulation.bin")
                .is_some(),
            "override for regulation.bin was not found"
        );
        assert!(
            asset_mapping
                .virtual_to_uid("data0:/event/common.emevd.dcx")
                .is_some(),
            "override for event/common.emevd.dcx not found"
        );
        assert!(
            asset_mapping
                .virtual_to_uid("data0:/common.emevd.dcx")
                .is_none(),
            "event/common.emevd.dcx was found incorrectly under the regulation root"
        );
    }
}
