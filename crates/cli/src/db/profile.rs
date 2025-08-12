use std::{fs::DirEntry, path::Path};

use color_eyre::eyre::{Context as _, OptionExt as _};
use me3_mod_protocol::{
    dependency::sort_dependencies,
    native::Native,
    package::{Package, WithPackageSource},
    Game, ModProfile,
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
    base_dir: Box<Path>,
    profile: ModProfile,
}

impl Profile {
    /// Create a new transient (in-memory) profile with no backing me3 file.
    pub fn transient() -> Self {
        Self {
            name: "transient-profile".to_string(),
            base_dir: Box::from(Path::new(".")),
            profile: Default::default(),
        }
    }

    /// Get the name of this profile. Defaults to the file name without the .me3 extension.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the directory containing this profile file.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Get the single game this profile supports, or None if it supports multiple games/omits
    /// support metadata.
    pub fn supported_game(&self) -> Option<Game> {
        let supports = self.profile.supports();

        if supports.len() == 1 {
            Some(supports[0].game)
        } else {
            None
        }
    }

    /// Get an unordered list of natives to be loaded by this profile.
    ///
    /// See [compile] to produce an ordered list.
    pub fn natives(&self) -> impl Iterator<Item = Native> {
        self.profile.natives().into_iter()
    }

    /// Get an unordered list of packages loaded by this profile.
    ///
    /// See [compile] to produce an ordered list.
    pub fn packages(&self) -> impl Iterator<Item = Package> {
        self.profile.packages().into_iter()
    }

    /// Returns misc. options set by this profile.
    pub fn options(&self) -> ProfileOptions {
        ProfileOptions {
            start_online: self.profile.start_online(),
        }
    }

    /// Compile this profile into a load order of native DLLs and packages to be loaded.
    pub fn compile(&self) -> color_eyre::Result<(Vec<Native>, Vec<Package>)> {
        fn exists<S: WithPackageSource>(p: &S) -> bool {
            match p.source().try_exists() {
                Ok(true) => true,
                _ => {
                    warn!(path = %p.source().display(), "specified path does not exist or is inaccessible");
                    false
                }
            }
        }

        fn canonicalize<S: WithPackageSource>(base_dir: &Path, sources: &mut Vec<S>) {
            sources
                .iter_mut()
                .for_each(|pkg| pkg.source_mut().make_absolute(base_dir));
            sources.retain(exists);
        }

        let mut packages = self.profile.packages();
        let mut natives = self.profile.natives();

        canonicalize(&self.base_dir, &mut packages);
        canonicalize(&self.base_dir, &mut natives);

        let ordered_natives = sort_dependencies(natives)?;
        let ordered_packages = sort_dependencies(packages)?;

        Ok((ordered_natives, ordered_packages))
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
    pub fn load(&self, path: impl AsRef<Path>) -> color_eyre::Result<Profile> {
        let path = path.as_ref();
        let is_file_ref = path.is_absolute() && path.exists();
        let canonical_path = is_file_ref
            .then_some(Box::from(path))
            .or_else(|| {
                self.search_paths
                    .iter()
                    .filter_map(|dir| {
                        let mut candidate = dir.join(path);
                        let _ = candidate.set_extension("me3");

                        candidate.exists().then_some(candidate.into_boxed_path())
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

        let parent = normalized_path
            .parent()?
            .map(|base| base.as_path())
            .ok_or_eyre("parent folder of mod profile is inaccessible")?;

        let profile = ModProfile::from_file(&canonical_path)?;
        let name = canonical_path
            .file_stem()
            .expect("BUG: profile path must have a filename")
            .to_string_lossy();

        Ok(Profile {
            name: name.to_string(),
            base_dir: Box::from(parent),
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
        assert_eq!(temp_file.parent().unwrap(), profile.base_dir());

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
