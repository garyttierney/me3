use std::{
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
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
            .env("SteamAppId", "1245620")
            .creation_flags(0)
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

        let response = payload.call(&request)?.inspect_err(|e| info!("{:#?}", e)).map_err(|e| eyre::eyre!(e.0))?;

        info!("Successfully attached");

        Ok(response)
    }

    // pub fn metadata(&self) -> LauncherResult<ImageMetadata> {
    //     let mut pbi: PROCESS_BASIC_INFORMATION = unsafe { mem::zeroed() };

    //     unsafe {
    //         let result = NtQueryInformationProcess(
    //             self.info.hProcess,
    //             ProcessBasicInformation,
    //             addr_of_mut!(pbi).cast(),
    //             size_of::<PROCESS_BASIC_INFORMATION>() as u32,
    //             null_mut(),
    //         );

    //         if result == 0 {
    //             bail!("NtQueryInformationProcess");
    //         }

    //         let peb = remote_ptr::read_ptr(self.info.hProcess, pbi.PebBaseAddress)?;

    //         ImageMetadata::new(self.info.hProcess, peb)
    //     }
    //     // Pointer to IMAGE_DOS_HEADER is at PEB+0x10
    // }

    // pub fn virtual_alloc(&self, requested_size: usize) -> LauncherResult<RemoteAllocator> {
    //     let allocation = unsafe {
    //         VirtualAllocEx(
    //             self.info.hProcess,
    //             null(),
    //             requested_size,
    //             MEM_COMMIT | MEM_RESERVE,
    //             PAGE_READWRITE,
    //         )
    //     };

    //     if allocation.is_null() {
    //         bail!("VirtualAllocEx failure");
    //     }

    //     let mut mbi: MEMORY_BASIC_INFORMATION = unsafe { mem::zeroed() };
    //     let mbi_read = unsafe {
    //         VirtualQueryEx(
    //             self.info.hProcess,
    //             allocation,
    //             &mut mbi,
    //             size_of::<MEMORY_BASIC_INFORMATION>(),
    //         )
    //     };

    //     if mbi_read == 0 {
    //         bail!("VirtualQueryEx failure");
    //     }

    //     Ok(RemoteAllocator::new(
    //         self.info.hProcess,
    //         allocation.cast(),
    //         mbi.RegionSize,
    //     ))
    // }

    pub fn join(mut self) {
        let _ = self.child.wait();
    }
}
