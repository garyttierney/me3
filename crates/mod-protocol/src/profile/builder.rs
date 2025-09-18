use std::{
    io,
    path::{Path, PathBuf},
};

use crate::{
    profile::{
        v2::{ModEntryV2, ModProfileV2},
        ModProfile,
    },
    Game,
};

#[derive(Default)]
pub struct ModProfileBuilder {
    supports: Option<Game>,
    mods: Vec<ModEntryV2>,
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
            mods,
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

        for mod_entry in mods {
            profile.push_mod_entry(mod_entry);
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
        self.mods.extend(iter.into_iter().map(Into::into));
        self
    }

    #[inline]
    pub fn with_mods<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item: Into<ModEntryV2>>,
    {
        self.mods.extend(iter.into_iter().map(Into::into));
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
