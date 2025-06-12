use std::mem;

use windows::{
    core::{s, w},
    Wdk::System::Threading::{NtQueryInformationProcess, PROCESSINFOCLASS},
    Win32::System::{
        Diagnostics::Debug::IsDebuggerPresent,
        LibraryLoader::{GetModuleHandleW, GetProcAddress},
        Threading::GetCurrentProcess,
    },
};

pub fn suspend_for_debugger() {
    let is_debugger_present = if is_under_wine() {
        is_wine_debugger_present
    } else {
        is_windows_debugger_present
    };

    while !is_debugger_present() {
        std::thread::sleep(std::time::Duration::from_secs(1));
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
