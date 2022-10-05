use std::{collections::BTreeMap, ffi::OsString, path::Path, sync::RwLock};

use once_cell::sync::OnceCell;
use walkdir::WalkDir;

use super::{FrameworkError, FrameworkGlobal};

mod hook;

pub struct VirtualFileSystem {
    overrides: RwLock<BTreeMap<OsString, OsString>>,
}

impl FrameworkGlobal for VirtualFileSystem {
    fn cell() -> &'static OnceCell<Self> {
        static INSTANCE: OnceCell<VirtualFileSystem> = OnceCell::new();
        &INSTANCE
    }

    fn create() -> Result<Self, FrameworkError> {
        hook::install_vfs_hooks()?;

        Ok(VirtualFileSystem {
            overrides: RwLock::new(BTreeMap::default()),
        })
    }
}

impl VirtualFileSystem {
    pub fn add_override_root<P: AsRef<Path>>(&self, path: P) {
        let mut overrides = self.overrides.write().unwrap();
        let root = path.as_ref();
        for entry in WalkDir::new(root) {
            match entry {
                Ok(dir_entry) => {
                    let override_path = dir_entry.path();
                    let overridden_path = override_path
                        .strip_prefix(root)
                        .expect("override entry was outside of override root");

                    log::trace!("overriding {:?} with {:?}", overridden_path, override_path);

                    overrides.insert(
                        overridden_path.as_os_str().to_owned(),
                        override_path.as_os_str().to_owned(),
                    );
                }
                Err(e) => {
                    log::warn!("inaccessible file in vfs override root: {:#?}", e)
                }
            }
        }

        log::info!("vfs override added for {:?}", root);
    }

    pub fn find_override(&self, path: &OsString) -> Option<OsString> {
        Some(self.overrides.read().unwrap().get(path)?.clone())
    }
}
