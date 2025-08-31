use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use chrono::Local;

pub struct LogsDb {
    base_dir: Box<Path>,
    retention: usize,
}

impl LogsDb {
    pub fn new<P: Into<Box<Path>>>(path: P) -> Self {
        Self {
            base_dir: path.into(),
            retention: 5,
        }
    }

    pub fn create_log_file(&self, profile_name: &str) -> color_eyre::Result<Box<Path>> {
        let profile_log_folder = self.base_dir.join(profile_name);
        fs::create_dir_all(&profile_log_folder)?;

        let log_files: Vec<(SystemTime, PathBuf)> = fs::read_dir(&profile_log_folder)
            .map(|dir| {
                dir.filter_map(|entry| {
                    let entry = entry.ok()?;
                    let metadata = entry.metadata().ok()?;
                    if metadata.is_file()
                        && entry.path().extension().is_some_and(|ext| ext == "log")
                    {
                        Some((metadata.modified().ok()?, entry.path()))
                    } else {
                        None
                    }
                })
                .collect()
            })
            .unwrap_or_default();

        if log_files.len() >= self.retention {
            if let Some((_, path_to_delete)) = log_files.iter().min_by_key(|(time, _)| *time) {
                let _ = fs::remove_file(path_to_delete);
            }
        }

        let now = Local::now();
        let log_file_suffix = now.format("%Y-%m-%d_%H-%M-%S").to_string();
        let log_file_path = profile_log_folder.join(format!("{log_file_suffix}.log"));

        Ok(log_file_path.into_boxed_path())
    }
}
