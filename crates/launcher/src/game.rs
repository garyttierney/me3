use std::{
    os::windows::{io::AsHandle, process::CommandExt},
    path::{Path, PathBuf},
    process::Command,
};

use dll_syringe::{
    process::{OwnedProcess, Process},
    rpc::RemotePayloadProcedure,
    Syringe,
};
use eyre::{eyre, OptionExt};
use me3_env::{deserialize_from_env, serialize_into_command, TelemetryVars};
use me3_launcher_attach_protocol::{AttachFunction, AttachRequest, Attachment};
use tracing::{info, instrument};
use windows::Win32::{
    Foundation::{
        CloseHandle, DuplicateHandle, DBG_CONTINUE, DUPLICATE_SAME_ACCESS,
        ERROR_ELEVATION_REQUIRED, WIN32_ERROR,
    },
    System::{
        Diagnostics::Debug::{
            ContinueDebugEvent, DebugActiveProcessStop, WaitForDebugEvent,
            CREATE_PROCESS_DEBUG_EVENT, DEBUG_EVENT,
        },
        Threading::{GetCurrentProcess, ResumeThread, SuspendThread, DEBUG_PROCESS, INFINITE},
    },
};

use crate::LauncherResult;

#[derive(Debug)]
pub struct Game {
    child: std::process::Child,
}

impl Game {
    #[instrument(err)]
    pub fn launch(game_binary: &Path, game_directory: Option<&Path>) -> LauncherResult<Self> {
        let mut command = Command::new(game_binary);
        command.current_dir(
            game_directory
                .map(Path::to_path_buf)
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or(PathBuf::from(".")),
        );

        let mut telemetry_vars: TelemetryVars = deserialize_from_env()?;
        telemetry_vars.trace_id = me3_telemetry::trace_id();

        info!(trace_id = telemetry_vars.trace_id, "game trace_id");
        serialize_into_command(telemetry_vars, &mut command);

        command.creation_flags(DEBUG_PROCESS.0);

        let child = command.spawn().map_err(|e| match e.raw_os_error().map(|i| WIN32_ERROR(i as u32)) {
            Some(ERROR_ELEVATION_REQUIRED) => eyre!(
                "Elevation is required to launch the game. Disable \"Run this program as an administrator\" and try again."
            ),
            _ => e.into()
        })?;

        Ok(Self { child })
    }

    #[instrument(skip_all, err)]
    pub fn attach(&self, dll_path: &Path, request: AttachRequest) -> LauncherResult<Attachment> {
        let pid = self.child.id();

        info!(pid, "attaching to process");

        let thread_handle = unsafe {
            let mut debug_event = DEBUG_EVENT::default();

            WaitForDebugEvent(&mut debug_event, INFINITE)?;

            assert_eq!(debug_event.dwDebugEventCode, CREATE_PROCESS_DEBUG_EVENT);

            // SAFETY: debug event code asserted above
            let event_info = &debug_event.u.CreateProcessInfo;

            // https://learn.microsoft.com/en-us/windows/win32/api/debugapi/nf-debugapi-waitfordebugevent
            CloseHandle(event_info.hFile)?;

            let current_process_handle = GetCurrentProcess();
            let mut thread_handle = Default::default();

            DuplicateHandle(
                current_process_handle,
                event_info.hThread,
                current_process_handle,
                &mut thread_handle,
                0,
                false,
                DUPLICATE_SAME_ACCESS,
            )?;

            // Increment the thread's suspend counter
            SuspendThread(event_info.hThread);

            ContinueDebugEvent(pid, debug_event.dwThreadId, DBG_CONTINUE)?;

            DebugActiveProcessStop(pid)?;

            thread_handle
        };

        let process_handle = self.child.as_handle().try_clone_to_owned()?;

        // SAFETY: `process_handle` is a process handle that is exclusively owned.
        let process = unsafe { OwnedProcess::from_handle_unchecked(process_handle) };

        let injector = Syringe::for_process(process);
        let module = injector.inject(dll_path)?;
        let payload: RemotePayloadProcedure<AttachFunction> = unsafe {
            injector
                .get_payload_procedure::<AttachFunction>(module, "me_attach")?
                .ok_or_eyre("No symbol named `me_attach` found")?
        };

        if request.config.suspend {
            info!("Process will be suspended until a debugger is attached...");
        }

        let response = payload.call(&request)?.map_err(|e| eyre::eyre!(e.0))?;

        unsafe {
            ResumeThread(thread_handle);
        }

        info!("Successfully attached");

        Ok(response)
    }

    pub fn join(mut self) {
        let _ = self.child.wait();
    }
}
