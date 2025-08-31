pub mod logs;
pub mod profile;
use std::path::Path;

pub use profile::ProfileDb;

use crate::{config::Config, db::logs::LogsDb};

pub struct DbContext {
    pub(crate) profiles: ProfileDb,
    pub(crate) logs: LogsDb,
}

impl DbContext {
    pub fn new(config: &Config) -> Self {
        let profile_search_paths = [
            config.profile_dir(),
            std::env::current_dir()
                .map(|path| path.into_boxed_path())
                .ok(),
        ];

        let logs = LogsDb::new(config.log_dir().unwrap_or(Box::from(Path::new("me3-logs"))));
        let profiles = ProfileDb::new(profile_search_paths.into_iter().flatten());

        Self { logs, profiles }
    }
}
