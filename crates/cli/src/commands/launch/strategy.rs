pub mod compat_tool;
pub mod direct;

use std::{ffi::OsString, path::Path, process::Command};

pub trait LaunchStrategy {
    fn build_command(self, exe: &Path, args: Vec<OsString>) -> color_eyre::Result<Command>;
}
