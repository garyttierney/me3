use std::{
    ffi::OsStr,
    fs::DirEntry,
    path::{Path, PathBuf},
};

use color_eyre::eyre::Context;
use me3_mod_protocol::{
    mod_file::{AsModFile, ModFile},
    native::Native,
    package::Package,
    profile::{ModProfile, ProfileMergeError},
    Game,
};
use normpath::PathExt;
use tracing::warn;

use crate::commands::profile::ProfileOptions;

pub struct ProfileDb {
    search_paths: Vec<Box<Path>>,
}

impl ProfileDb {
    pub fn new<P: AsRef<Path>>(search_paths: impl Iterator<Item = P>) -> Self {
        Self {
            search_paths: search_paths.map(|path| Box::from(path.as_ref())).collect(),
        }
    }
}

pub struct Profile {
    name: String,
    path: PathBuf,
    profile: ModProfile,
}

impl Profile {
    /// Create a new transient (in-memory) profile with no backing me3 file.
    pub fn transient() -> Self {
        Self {
            name: "transient-profile".to_string(),
            path: Default::default(),
            profile: Default::default(),
        }
    }

    /// Get the name of this profile. Defaults to the file name without the .me3 extension.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the directory containing this profile file.
    pub fn base_dir(&self) -> Option<&Path> {
        self.path.parent()
    }

    /// Returns the path to the profile file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the single game this profile supports, or None if it supports multiple games/omits
    /// support metadata.
    pub fn supported_game(&self) -> Option<Game> {
        self.profile.game()
    }

    /// Returns a list of natives to be loaded by this profile.
    pub fn natives(&self) -> impl Iterator<Item = Native> {
        self.profile.natives().into_iter()
    }

    /// Returns a list of packages loaded by this profile.
    pub fn packages(&self) -> impl Iterator<Item = Package> {
        self.profile.packages().into_iter()
    }

    /// Returns a list of profiles loaded by this profile.
    pub fn profiles(&self) -> impl Iterator<Item = ModFile> {
        self.profile.profiles().into_iter()
    }

    /// Get the savefile name that may be overridden by this profile.
    pub fn savefile(&self) -> Option<String> {
        self.profile.savefile()
    }

    /// Returns misc. options set by this profile.
    pub fn options(&self) -> ProfileOptions {
        ProfileOptions {
            start_online: self.profile.start_online(),
            disable_arxan: self.profile.disable_arxan(),
        }
    }

    /// Attempt to apply the properties of another profile on top of this profile.
    ///
    /// Returns a profile that is a combination of both.
    pub fn try_merge<P: AsRef<ModProfile>>(&self, other: &P) -> Result<Self, ProfileMergeError> {
        Ok(Self {
            name: self.name.clone(),
            path: self.path.clone(),
            profile: self.profile.try_merge(other.as_ref())?,
        })
    }

    /// Compile this profile into a load order of native DLLs, packages and files to be loaded.
    pub fn compile(&self) -> color_eyre::Result<(Vec<Native>, Vec<Package>)> {
        fn canonicalize<S: AsModFile>(base_dir: &Path, sources: &mut Vec<S>) {
            sources
                .iter_mut()
                .for_each(|i| i.as_mod_file_mut().make_absolute(base_dir));

            sources.retain(|s| match s.as_mod_file().as_ref().try_exists() {
                Ok(true) => s.as_mod_file().enabled,
                _ => {
                    warn!(
                        "path" = ?s.as_mod_file().as_ref(),
                        "specified path does not exist or is inaccessible"
                    );
                    false
                }
            });
        }

        let mut packages = self.profile.packages();
        let mut natives = self.profile.natives();

        let base_dir = self.base_dir().unwrap_or(Path::new("."));

        canonicalize(base_dir, &mut packages);
        canonicalize(base_dir, &mut natives);

        Ok((natives, packages))
    }
}

impl AsRef<ModProfile> for Profile {
    fn as_ref(&self) -> &ModProfile {
        &self.profile
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ProfileDbError {
    #[error("no profile named {0} could be found")]
    MissingProfileFile(Box<Path>),

    #[error("unexpected IO error reading {path}: {inner}")]
    Other {
        path: Box<Path>,
        inner: std::io::Error,
    },
}

impl ProfileDb {
    pub fn load<P: AsRef<Path>>(&self, path: P) -> color_eyre::Result<Profile> {
        let path = path.as_ref();
        let is_file_ref = path.is_absolute() && path.exists();
        let canonical_path = is_file_ref
            .then_some(Box::from(path))
            .or_else(|| {
                self.search_paths
                    .iter()
                    .filter_map(|dir| {
                        let mut candidate = dir.join(path);
                        let extension = candidate.extension();

                        if extension.is_none()
                            || (extension != Some(OsStr::new(".me3")) && !candidate.is_file())
                        {
                            candidate.as_mut_os_string().push(".me3");

                            if !candidate.exists() {
                                return None;
                            }
                        }

                        Some(candidate.into_boxed_path())
                    })
                    .next_back()
            })
            .ok_or_else(|| ProfileDbError::MissingProfileFile(Box::from(path)))?;

        let normalized_path = canonical_path
            .normalize()
            .map_err(|inner| ProfileDbError::Other {
                path: Box::from(path),
                inner,
            })
            .wrap_err("failed while normalizing")?;

        let name = canonical_path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();

        let profile = ModProfile::from_file(&canonical_path)?;

        Ok(Profile {
            name,
            path: normalized_path.into_path_buf(),
            profile,
        })
    }

    pub fn list(&self) -> impl Iterator<Item = Box<Path>> {
        self.search_paths
            .iter()
            .filter_map(|dir| dir.read_dir().ok())
            .flatten()
            .filter_map(|entry: Result<DirEntry, _>| {
                let entry = entry.ok()?;
                let entry_path = entry.path();

                entry_path
                    .extension()
                    .is_some_and(|ext| ext == "me3")
                    .then(|| entry.path().into_boxed_path())
            })
    }
}

#[cfg(test)]
mod test {
    use std::error::Error;

    use assert_fs::prelude::{FileTouch, FileWriteStr, PathChild};

    use super::ProfileDb;

    #[test]
    fn lists_me3_files() -> Result<(), Box<dyn Error>> {
        let temp_dir = assert_fs::TempDir::new()?;
        temp_dir.child("my-profile.me3").touch()?;

        let db = ProfileDb {
            search_paths: vec![Box::from(temp_dir.path())],
        };

        let profiles: Vec<_> = db.list().collect();
        let profile = &profiles[0];
        let profile_name = profile
            .file_name()
            .expect("returned profile had no filename");

        assert_eq!(1, profiles.len());
        assert_eq!("my-profile.me3", profile_name);
        assert!(profile.is_absolute());
        Ok(())
    }

    #[test]
    fn load_absolute_me3_file() -> Result<(), Box<dyn Error>> {
        let db = ProfileDb {
            search_paths: vec![],
        };
        let temp_file = assert_fs::NamedTempFile::new("my-profile.me3")?;
        temp_file.write_str(r#"profileVersion = 'v1'"#)?;
        let profile = db.load(temp_file.path())?;

        assert_eq!("my-profile", profile.name());
        assert_eq!(temp_file.parent(), profile.base_dir());

        Ok(())
    }

    #[test]
    pub fn load_relative_me3_file_from_search_path() -> Result<(), Box<dyn Error>> {
        let temp_dir = assert_fs::TempDir::new()?;
        temp_dir
            .child("my-profile.me3")
            .write_str(r#"profileVersion = 'v1'"#)?;

        let db = ProfileDb {
            search_paths: vec![Box::from(temp_dir.path())],
        };

        let profile = db.load("my-profile.me3")?;

        assert_eq!("my-profile", profile.name());

        Ok(())
    }

    #[test]
    pub fn load_relative_me3_profile_name_from_search_path() -> Result<(), Box<dyn Error>> {
        let temp_dir = assert_fs::TempDir::new()?;
        temp_dir
            .child("my-profile.me3")
            .write_str(r#"profileVersion = 'v1'"#)?;

        let db = ProfileDb {
            search_paths: vec![Box::from(temp_dir.path())],
        };

        let profile = db.load("my-profile")?;
        assert_eq!("my-profile", profile.name());

        Ok(())
    }
}
