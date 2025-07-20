use std::path::{Path, PathBuf};

use color_eyre::Result;
use directories::ProjectDirs;

#[derive(Debug)]
pub struct KnownDirs {
    /// The current working directory we launched from.
    cwd: Option<Box<Path>>,

    /// The directory containing the binary we launched.
    exe_dir: Option<Box<Path>>,

    /// Linux installation prefix (defaults to /)
    #[cfg(target_os = "linux")]
    prefix: Option<Box<Path>>,

    /// Windows installation directory
    #[cfg(target_os = "windows")]
    pub(crate) installation: Option<Box<Path>>,

    project_dirs: Option<ProjectDirs>,
}

pub trait OptionalPathExt {
    fn join<P>(&self, path: P) -> Option<Box<Path>>
    where
        P: AsRef<Path>;
}

impl<S: AsRef<Path>> OptionalPathExt for Option<S> {
    fn join<P>(&self, path: P) -> Option<Box<Path>>
    where
        P: AsRef<Path>,
    {
        self.as_ref()
            .map(|parent| parent.as_ref().join(path).into_boxed_path())
    }
}

const PROJECT_QUALIFIER: &str = "com.github";
const PROJECT_ORG: &str = "garyttierney";
const PROJECT_NAME: &str = "me3";

impl Default for KnownDirs {
    fn default() -> Self {
        #[cfg(target_os = "windows")]
        fn find_install_dir() -> Result<Box<Path>, color_eyre::Report> {
            use std::str::FromStr;

            use winreg::{enums::HKEY_CURRENT_USER, RegKey};

            let hklm = RegKey::predef(HKEY_CURRENT_USER);
            let me3_reg = hklm.open_subkey(format!(r"Software\{PROJECT_ORG}\{PROJECT_NAME}"))?;
            let install_dir_value = me3_reg.get_value::<String, _>("Install_Dir")?;
            let install_dir = PathBuf::from_str(&install_dir_value)?;

            Ok(install_dir.into_boxed_path())
        }

        Self {
            cwd: std::env::current_dir()
                .map(|cwd| cwd.into_boxed_path())
                .ok(),
            exe_dir: std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(Path::to_path_buf))
                .map(PathBuf::into_boxed_path),

            #[cfg(target_os = "linux")]
            prefix: Some(Box::from(Path::new("/"))),
            #[cfg(target_os = "windows")]
            installation: find_install_dir().ok(),
            project_dirs: ProjectDirs::from(PROJECT_QUALIFIER, PROJECT_ORG, PROJECT_NAME),
        }
    }
}

impl KnownDirs {
    /// Discover the cache directory. This location is used to store files that are used to speed up
    /// subsequent launches of me3.
    pub fn cache_dir(&self) -> Option<Box<Path>> {
        self.project_dirs
            .as_ref()
            .map(|dirs| Box::from(dirs.cache_dir()))
    }

    /// Discover the canonical directory for me3 profile files.
    pub fn profile_dir(&self) -> Option<Box<Path>> {
        self.project_dirs
            .as_ref()
            .map(|dir| dir.config_local_dir().join("profiles").into_boxed_path())
    }

    /// Discover the data directory. This location is used to store log files.
    pub fn data_dir(&self) -> Option<Box<Path>> {
        self.project_dirs
            .as_ref()
            .map(|dirs| Box::from(dirs.data_local_dir()))
            .or(self.cwd.clone())
    }

    /// Discover candidate paths to me3 Windows binary directories, ordered from least priority to
    /// highest.
    ///
    /// Under Windows this is the directory containing the current executable.
    #[cfg(target_os = "windows")]
    pub fn windows_bin_dirs(&self) -> impl Iterator<Item = Box<Path>> {
        std::iter::once(self.exe_dir.clone()).flatten()
    }

    /// Discover candidate paths to me3 Windows binary directories, ordered from least priority to
    /// highest.
    ///
    /// Under Linux these can be one of the following:
    /// - $PREFIX/usr/lib/me3/x6_64-windows
    /// - $EXE_DIR/win64
    /// - {$XDG_DATA_DIR:=$HOME/.local/share}/me3/windows-bin
    /// - $EXE_DIR/../../x86_64-pc-windows-msvc/debug/ (Debug binaries only)
    /// - $EXE_DIR/../../x86_64-pc-windows-msvc/release/
    #[cfg(target_os = "linux")]
    pub fn windows_bin_dirs(&self) -> impl Iterator<Item = Box<Path>> {
        // $GIT_ROOT/target
        let potential_target_dir = self.cwd.join("target");

        let windows_bin_dirs = [
            self.prefix.join("usr/lib/me3/x86_64-windows"),
            self.project_dirs
                .as_ref()
                .map(|dirs| dirs.data_local_dir().join("windows-bin").into_boxed_path()),
            self.exe_dir.join("win64"), // Portable distribution
            #[cfg(debug_assertions)]
            potential_target_dir.join("x86_64-pc-windows-msvc/debug"),
            potential_target_dir.join("x86_64-pc-windows-msvc/release"),
        ];

        windows_bin_dirs.into_iter().flatten()
    }

    /// Discover the candidate paths to me3 configuration directories, ordered from least priority
    /// to highest.
    ///
    /// These can be one of the following:
    ///
    /// - $PREFIX/me3 (Linux)
    /// - %INSTALLDIR%/config (Windows)
    /// - $XDG_CONFIG_DIR:=$HOME/.config/me3 (Linux)
    /// - %LOCALAPPDATA%/garyttierney/me3/config (Windows)
    /// - ./me3.toml
    pub fn config_dirs(&self) -> impl Iterator<Item = Box<Path>> {
        let config_dirs = [
            #[cfg(target_os = "linux")]
            self.prefix.join("etc/me3"),
            #[cfg(target_os = "windows")]
            self.installation.join("config"),
            self.cwd.clone(),
            self.project_dirs
                .as_ref()
                .map(|proj| Box::from(proj.config_local_dir())),
        ];

        config_dirs.into_iter().flatten()
    }
}
