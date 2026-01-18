use std::process::Command;

use crate::commands::launch::strategy::LaunchStrategy;

#[derive(Debug)]
pub struct DirectLaunchStrategy;

impl LaunchStrategy for DirectLaunchStrategy {
    fn build_command(
        self,
        exe: &std::path::Path,
        args: Vec<std::ffi::OsString>,
    ) -> color_eyre::Result<Command> {
        let mut command = Command::new(exe);
        command.args(args);

        Ok(command)
    }
}
