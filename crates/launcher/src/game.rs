use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use dll_syringe::{
    process::{OwnedProcess, Process},
    rpc::RemotePayloadProcedure,
    Syringe,
};
use eyre::OptionExt;
use me3_launcher_attach_protocol::{AttachFunction, AttachRequest, Attachment};
use tracing::{info, instrument};

use crate::LauncherResult;

#[derive(Debug)]
pub struct Game {
    child: std::process::Child,
}

impl Game {
    #[instrument]
    pub fn launch(game_binary: &Path, game_directory: Option<&Path>) -> LauncherResult<Self> {
        let child = Command::new(game_binary)
            .current_dir(
                game_directory
                    .map(Path::to_path_buf)
                    .or_else(|| std::env::current_dir().ok())
                    .unwrap_or(PathBuf::from(".")),
            )
            // FIXME
            .stdout(Stdio::inherit())
            .stdin(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        Ok(Self { child })
    }

    #[instrument]
    pub fn attach(
        &mut self,
        dll_path: &Path,
        request: AttachRequest,
    ) -> LauncherResult<Attachment> {
        let pid = self.child.id();

        info!("Attaching to process {pid}");

        let process = OwnedProcess::from_pid(pid)?;

        // TODO: no hardcoded timeout.
        let _ = process.wait_for_module_by_name("kernel32", Duration::from_secs(5));
        let injector = Syringe::for_process(process);
        let module = injector.inject(dll_path)?;
        let payload: RemotePayloadProcedure<AttachFunction> = unsafe {
            injector
                .get_payload_procedure::<AttachFunction>(module, "me_attach")?
                .ok_or_eyre("No symbol named `me_attach` found")?
        };

        let response = payload
            .call(&request)?
            .inspect_err(|e| info!("{:#?}", e))
            .map_err(|e| eyre::eyre!(e.0))?;

        info!("Successfully attached");

        Ok(response)
    }

    pub fn join(mut self) {
        let _ = self.child.wait();
    }
}
