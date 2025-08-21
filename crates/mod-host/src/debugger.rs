use std::{
    mem,
    os::{raw::c_void, windows::raw::HANDLE},
};

use eyre::OptionExt;
use windows::{
    core::{s, w},
    Wdk::System::Threading::{
        NtQueryInformationProcess, ThreadHideFromDebugger, PROCESSINFOCLASS, THREADINFOCLASS,
    },
    Win32::{
        Foundation::{NTSTATUS, STATUS_SUCCESS},
        System::{
            Diagnostics::Debug::IsDebuggerPresent,
            LibraryLoader::{GetModuleHandleW, GetProcAddress},
            Threading::GetCurrentProcess,
        },
    },
};

use crate::host::hook::HookInstaller;

pub fn suspend_for_debugger() {
    while !is_debugger_present() {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

pub fn prevent_hiding_threads() -> Result<(), eyre::Error> {
    type NtSetInformationThread = unsafe extern "C" fn(
        threadhandle: HANDLE,
        threadinformationclass: THREADINFOCLASS,
        threadinformation: *const c_void,
        threadinformationlength: u32,
    ) -> NTSTATUS;

    let nt_set_information_thread = unsafe {
        let ntdll = GetModuleHandleW(w!("ntdll.dll"))?;
        mem::transmute::<_, NtSetInformationThread>(
            GetProcAddress(ntdll, s!("NtSetInformationThread"))
                .ok_or_eyre("NtSetInformationThread not found")?,
        )
    };

    // Ignore ThreadHideFromDebugger calls to NtSetInformationThread.
    let hook = HookInstaller::new(nt_set_information_thread)
        .with_closure(|p1, threadinformationclass, p3, p4, trampoline| unsafe {
            if threadinformationclass == ThreadHideFromDebugger {
                return STATUS_SUCCESS;
            }

            trampoline(p1, threadinformationclass, p3, p4)
        })
        .install()?;

    mem::forget(hook);

    Ok(())
}

pub fn is_debugger_present() -> bool {
    if is_under_wine() {
        is_wine_debugger_present()
    } else {
        is_windows_debugger_present()
    }
}

fn is_windows_debugger_present() -> bool {
    unsafe { IsDebuggerPresent().as_bool() }
}

fn is_wine_debugger_present() -> bool {
    is_windows_debugger_present() || is_ptraced()
}

// https://github.com/ValveSoftware/Proton/blob/3a269ab9966409b968c8bc8f3e68bd0d2f42aadf/steam_helper/steam.c#L772-L782
fn is_ptraced() -> bool {
    unsafe {
        let mut pid = 0u32;
        let mut len = 0u32;

        NtQueryInformationProcess(
            GetCurrentProcess(),
            PROCESSINFOCLASS(1100), // ProcessWineUnixDebuggerPid
            &raw mut pid as *mut _,
            mem::size_of::<u32>() as u32,
            &mut len,
        )
        .is_ok()
            && pid != 0
    }
}

fn is_under_wine() -> bool {
    unsafe {
        GetModuleHandleW(w!("ntdll.dll"))
            .is_ok_and(|h| GetProcAddress(h, s!("wine_get_version")).is_some())
    }
}
