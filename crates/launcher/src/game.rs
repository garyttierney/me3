use std::{
    ffi::c_void,
    io::Write,
    iter, mem,
    os::windows::{
        ffi::OsStrExt,
        io::{AsHandle, AsRawHandle, OwnedHandle},
        process::{ChildExt, CommandExt},
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
};

use eyre::{eyre, Context};
use me3_env::{deserialize_from_env, serialize_into_command, TelemetryVars};
use me3_ipc::{bridge::BridgeToChild, message::MsgToParent, request::Response};
use me3_launcher_attach_protocol::{AttachRequest, Attachment};
use tracing::{error, info, instrument};
use tracing_subscriber::fmt::MakeWriter;
use windows::{
    core::{s, w, Error as WinError},
    Win32::{
        Foundation::{CloseHandle, ERROR_ELEVATION_REQUIRED, HANDLE, WAIT_OBJECT_0, WIN32_ERROR},
        System::{
            Diagnostics::Debug::WriteProcessMemory,
            LibraryLoader::{GetModuleHandleW, GetProcAddress},
            Memory::{VirtualAllocEx, MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE},
            Threading::{
                CreateRemoteThread, ResumeThread, WaitForSingleObject, CREATE_SUSPENDED, INFINITE,
            },
        },
    },
};

use crate::{writer::MakeWriterWrapper, LauncherResult};

pub struct Game {
    pub(crate) child: std::process::Child,
    pub(crate) bridge: Arc<BridgeToChild>,
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

        info!(trace_id = ?telemetry_vars.trace_id);
        serialize_into_command(telemetry_vars, &mut command);

        command.creation_flags(CREATE_SUSPENDED.0);

        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let bridge = Arc::new(me3_ipc::bridge::to_child(32, &mut command)?);

        let child = command.spawn().map_err(|e| match e.raw_os_error().map(|i| WIN32_ERROR(i as u32)) {
            Some(ERROR_ELEVATION_REQUIRED) => eyre!(
                "Elevation is required to launch the game. Disable \"Run this program as an administrator\" and try again."
            ),
            _ => e.into()
        })?;

        Ok(Self { child, bridge })
    }

    #[instrument(skip_all, err)]
    pub fn attach(
        &self,
        dll_path: &Path,
        console_log: MakeWriterWrapper,
        file_log: MakeWriterWrapper,
        attach_request: AttachRequest,
    ) -> LauncherResult<Attachment> {
        let pid = self.child.id();

        info!(pid, "attaching to process");

        self.spawn_msg_thread(console_log, file_log);

        let thread_handle = self.child.main_thread_handle();
        let process_handle = self.child.as_handle().try_clone_to_owned()?;

        inject_dll(&process_handle, dll_path).wrap_err("failed to inject mod host DLL")?;

        if attach_request.config.suspend {
            info!("Process will be suspended until a debugger is attached...");
        }

        let response = self
            .bridge
            .request(attach_request)?
            .map_err(|e| eyre!(e.0))?;

        unsafe {
            ResumeThread(HANDLE(thread_handle.as_raw_handle()));
        }

        info!("Successfully attached");

        Ok(response)
    }

    pub fn join(mut self) {
        let _ = self.child.wait();
    }

    fn spawn_msg_thread(&self, console_log: MakeWriterWrapper, file_log: MakeWriterWrapper) {
        let bridge = self.bridge.clone();
        std::thread::spawn(move || {
            let recv_span = bridge.enter_recv_span().unwrap();

            loop {
                let msg = match recv_span.recv() {
                    Ok(msg) => msg,
                    Err(error) => {
                        error!(%error, "failed to receive message");
                        continue;
                    }
                };

                match msg {
                    MsgToParent::Response(res) => Response::forward(res),
                    MsgToParent::ConsoleLog(s) => {
                        let _ = console_log.make_writer().write_all(s.as_bytes());
                    }
                    MsgToParent::FileLog(s) => {
                        let _ = file_log.make_writer().write_all(s.as_bytes());
                    }
                }
            }
        });
    }
}

fn inject_dll(process: &OwnedHandle, path: &Path) -> LauncherResult<()> {
    let path = path
        .as_os_str()
        .encode_wide()
        .chain(iter::once(b'\0' as u16))
        .collect::<Vec<_>>();

    unsafe {
        let process_handle = HANDLE(process.as_raw_handle());

        let kernel32 = GetModuleHandleW(w!("kernel32.dll"))?;
        let load_library = GetProcAddress(kernel32, s!("LoadLibraryW"));

        let path_str = VirtualAllocEx(
            process_handle,
            None,
            path.len() * 2,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        );

        WriteProcessMemory(
            process_handle,
            path_str,
            path.as_ptr() as *const c_void,
            path.len() * 2,
            None,
        )?;

        let thread = CreateRemoteThread(
            process_handle,
            None,
            0,
            Some(mem::transmute(load_library)),
            Some(path_str),
            0,
            None,
        )?;

        if WaitForSingleObject(thread, INFINITE) != WAIT_OBJECT_0 {
            return Err(WinError::from_thread().into());
        }

        CloseHandle(thread)?;
    }

    Ok(())
}
