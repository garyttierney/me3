use std::{
    fs::OpenOptions,
    mem,
    os::windows::{
        io::{AsHandle, AsRawHandle},
        process::{ChildExt, CommandExt},
    },
    path::{Path, PathBuf},
    process::Command,
};

use dll_syringe::{
    process::{
        memory::{ProcessMemoryBuffer, ProcessMemorySlice},
        BorrowedProcess, OwnedProcess, Process,
    },
    Syringe,
};
use eyre::{eyre, OptionExt};
use me3_env::{deserialize_from_env, serialize_into_command, TelemetryVars};
use me3_launcher_attach_protocol::{AttachError, AttachRequest, Attachment};
use tracing::{info, instrument};
use windows::Win32::{
    Foundation::{ERROR_ELEVATION_REQUIRED, HANDLE, WIN32_ERROR},
    System::{
        Diagnostics::Debug::WriteProcessMemory,
        Memory::{VirtualAllocEx, MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE},
        Threading::{
            CreateRemoteThread, ResumeThread, WaitForSingleObject, CREATE_SUSPENDED, INFINITE,
        },
    },
};

use crate::LauncherResult;

#[derive(Debug)]
pub struct Game {
    pub(crate) child: std::process::Child,
}

impl Game {
    #[instrument(skip_all, err)]
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

        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&telemetry_vars.monitor_file_path)?;

        info!(trace_id = ?telemetry_vars.trace_id);
        serialize_into_command(telemetry_vars, &mut command);

        command.creation_flags(CREATE_SUSPENDED.0);
        command.stdout(log_file);

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

        let thread_handle = self.child.main_thread_handle();
        let process_handle = self.child.as_handle().try_clone_to_owned()?;

        // SAFETY: `process_handle` is a process handle that is exclusively owned.
        let process = unsafe { OwnedProcess::from_handle_unchecked(process_handle) };
        let injector = syringe_for_suspended_process(process)?;

        let module = injector.inject(dll_path)?;
        let procedure = unsafe {
            injector
                .get_raw_procedure::<extern "C" fn(*mut u8, usize) -> *mut u8>(module, "me_attach")?
                .ok_or_eyre("No symbol named `me_attach` found")?
        };

        let (attach_payload, attach_payload_len) =
            serialize_attach_payload(injector.process(), &request)?;

        if request.config.suspend {
            info!("Process will be suspended until a debugger is attached...");
        }

        let result_payload = procedure.call(attach_payload, attach_payload_len)?;
        let response = unsafe {
            deserialize_result_payload(injector.process(), result_payload)?
                .map_err(|e| eyre!(e.0))?
        };

        unsafe {
            ResumeThread(HANDLE(thread_handle.as_raw_handle()));
        }

        info!("Successfully attached");

        Ok(response)
    }

    pub fn join(mut self) {
        let _ = self.child.wait();
    }
}

fn syringe_for_suspended_process(process: OwnedProcess) -> LauncherResult<Syringe> {
    unsafe {
        let process_handle = HANDLE(process.as_raw_handle());

        let stub = VirtualAllocEx(
            process_handle,
            None,
            1,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        );

        static RET: u8 = 0xC3;

        WriteProcessMemory(process_handle, stub, &raw const RET as _, 1, None)?;

        let thread = CreateRemoteThread(
            process_handle,
            None,
            0,
            Some(mem::transmute(stub)),
            None,
            0,
            None,
        )?;

        WaitForSingleObject(thread, INFINITE);
    }

    Ok(Syringe::for_process(process))
}

fn serialize_attach_payload(
    process: BorrowedProcess<'_>,
    request: &AttachRequest,
) -> LauncherResult<(*mut u8, usize)> {
    let serialized = serde_json::to_string(request)?;
    let bytes = serialized.as_bytes();

    let buffer = ProcessMemoryBuffer::allocate_data(process, bytes.len())?;
    buffer.write(0, bytes)?;

    Ok((buffer.leak().as_ptr(), bytes.len()))
}

unsafe fn deserialize_result_payload(
    process: BorrowedProcess<'_>,
    result_payload: *mut u8,
) -> LauncherResult<Result<Attachment, AttachError>> {
    let payload_len = unsafe {
        ProcessMemorySlice::from_raw_parts(
            result_payload,
            mem::size_of::<usize>(),
            process,
        )
        .read_struct::<usize>(0)?
    };

    let mut bytes = Vec::new();
    bytes.resize(payload_len, b' ');

    unsafe {
        ProcessMemorySlice::from_raw_parts(
            result_payload.add(mem::size_of::<usize>()),
            payload_len,
            process,
        )
        .read(0, &mut bytes)?;
    };

    Ok(serde_json::from_slice(&bytes)?)
}
