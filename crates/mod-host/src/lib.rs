use std::sync::OnceLock;

use me3_launcher_attach_protocol::{AttachRequest, AttachResponse};

static INSTANCE: OnceLock<usize> = OnceLock::new();

dll_syringe::payload_procedure! {
    fn me_attach(_request: AttachRequest) -> AttachResponse {
        AttachResponse {  }
    }
}

/// https://learn.microsoft.com/en-us/windows/win32/dlls/dllmain#parameters
const DLL_PROCESS_ATTACH: u32 = 1;

#[no_mangle]
#[allow(unused)]
pub extern "stdcall" fn DllMain(instance: usize, reason: u32, _: *mut usize) -> i32 {
    if reason == DLL_PROCESS_ATTACH {
        let _ = INSTANCE.set(instance);
    }

    1
}
