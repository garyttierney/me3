#![feature(assert_matches)]

use windows::Win32::{
    Foundation::HINSTANCE,
    System::{
        LibraryLoader::{DisableThreadLibraryCalls, FreeLibraryAndExitThread},
        SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
    },
};

mod bootstrap;

#[no_mangle]
pub extern "stdcall" fn DllMain(hinst_dll: HINSTANCE, fdw_reason: u32, _: *const ()) -> i32 {
    let success = match fdw_reason {
        DLL_PROCESS_ATTACH => {
            unsafe {
                DisableThreadLibraryCalls(hinst_dll);
                let _ = std::thread::spawn(move || {
                    let exit_code = match std::panic::catch_unwind(bootstrap::setup_and_run) {
                        Err(e) => {
                            eprintln!("me3_host panicked in bootstrap: {:#?}", e);
                            0
                        }
                        Ok(Err(e)) => {
                            eprintln!("encountered an error during setup: {:#?}", e);
                            0
                        }
                        Ok(_) => 1,
                    };
                    FreeLibraryAndExitThread(hinst_dll, exit_code)
                });
            }

            true
        }
        DLL_PROCESS_DETACH => true,
        _ => true,
    };

    success as i32
}
