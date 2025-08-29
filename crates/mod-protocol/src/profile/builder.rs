use std::{
    io,
    path::{Path, PathBuf},
};

use crate::{
    mod_file::ModFile,
    profile::{
        v2::{ModProfileV2, ProfileDependency},
        ModProfile,
    },
    Game,
};

#[derive(Default)]
pub struct ModProfileBuilder {
    supports: Option<Game>,
    dependencies: Vec<(String, ProfileDependency)>,
    savefile: Option<String>,
    start_online: Option<bool>,
    disable_arxan: Option<bool>,
}

impl ModProfileBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&mut self) -> ModProfile {
        let Self {
            supports,
            dependencies,
            savefile,
            start_online,
            disable_arxan,
        } = std::mem::take(self);

        let mut profile = ModProfileV2 {
            supports,
            savefile,
            start_online,
            disable_arxan,
            ..Default::default()
        };

        for uses in dependencies {
            profile.push_dependency(uses);
        }

        ModProfile::V2(profile)
    }

    pub fn write<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let profile = self.build();
        let contents = toml::to_string_pretty(&profile).map_err(io::Error::other)?;
        std::fs::write(path, contents)
    }

    pub fn with_supported_game(&mut self, game: Option<Game>) -> &mut Self {
        self.supports = game;
        self
    }

    #[inline]
    pub fn with_paths<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = PathBuf>,
    {
        self.dependencies
            .extend(iter.into_iter().map(|i| ModFile::from(i).into()));
        self
    }

    #[inline]
    pub fn with_dependencies<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item: Into<(String, ProfileDependency)>>,
    {
        self.dependencies.extend(iter.into_iter().map(Into::into));
        self
    }

    pub fn with_savefile(&mut self, name: Option<String>) -> &mut Self {
        self.savefile = name;
        self
    }

    pub fn start_online(&mut self, start_online: Option<bool>) -> &mut Self {
        self.start_online = start_online;
        self
    }

    pub fn disable_arxan(&mut self, disable_arxan: Option<bool>) -> &mut Self {
        self.disable_arxan = disable_arxan;
        self
    }
}
