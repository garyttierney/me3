use std::{
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use crate::mapping::{VfsKey, VfsOverride};

pub struct SavefileOverrideMapping {
    savefile_dir: VfsKey,
    override_path: OnceLock<(VfsOverride, VfsOverride)>,
    on_override: Box<dyn Fn(&Path) -> PathBuf + Send + Sync>,
}

impl SavefileOverrideMapping {
    #[inline]
    pub fn new<P, F>(savefile_dir: P, f: F) -> io::Result<Self>
    where
        P: AsRef<Path>,
        F: Fn(&Path) -> PathBuf + Send + Sync + 'static,
    {
        Ok(Self {
            savefile_dir: VfsKey::for_disk_path(savefile_dir.as_ref())?,
            override_path: OnceLock::new(),
            on_override: Box::new(f),
        })
    }

    #[inline]
    pub fn try_override(&self, path: &Path, key: &VfsKey) -> Option<&VfsOverride> {
        if path.extension() != Some(OsStr::new("bak")) {
            self.try_override_inner(path, key).map(|(sl2, _)| sl2)
        } else {
            self.try_override_inner(&path.with_extension(""), key)
                .map(|(_, bak)| bak)
        }
    }

    #[inline]
    fn try_override_inner(&self, path: &Path, key: &VfsKey) -> Option<&(VfsOverride, VfsOverride)> {
        if path.extension() != Some(OsStr::new("sl2")) || !key.0.starts_with(&self.savefile_dir) {
            return None;
        }

        Some(self.override_path.get_or_init(|| {
            let override_path = (self.on_override)(path);

            let override_path_bak = {
                let mut override_path = override_path.clone();
                override_path.as_mut_os_string().push(".bak");
                override_path
            };

            (
                VfsOverride::new(override_path),
                VfsOverride::new(override_path_bak),
            )
        }))
    }
}
