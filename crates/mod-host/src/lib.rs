use std::ptr::null;

use me3_launcher_attach_protocol::{AttachRequest, AttachResponse};
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxA, MB_OK};

dll_syringe::payload_procedure! {
    fn me_attach(request: AttachRequest) -> AttachResponse {
        AttachResponse {  }
    }
}

#[no_mangle]
#[allow(unused)]
pub extern "stdcall" fn DllMain(hinstDLL: usize, dwReason: u32, lpReserved: *mut usize) -> i32 {
    1
}
