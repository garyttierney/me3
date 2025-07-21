pub mod profile;
pub use profile::ProfileDb;

use crate::config::Config;

pub struct DbContext {
    pub(crate) profiles: ProfileDb,
}

impl DbContext {
    pub fn new(config: &Config) -> Self {
        let profile_search_paths = [
            config.profile_dir(),
            std::env::current_dir()
                .map(|path| path.into_boxed_path())
                .ok(),
        ];

        let profiles = ProfileDb::new(profile_search_paths.into_iter().flatten());

        Self { profiles }
    }
}
