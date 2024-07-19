#![feature(fn_traits)]
#![feature(fn_ptr_trait)]
#![feature(tuple_trait)]
#![feature(unboxed_closures)]
#![feature(naked_functions)]

use std::sync::OnceLock;

use me3_launcher_attach_protocol::{AttachRequest, AttachResult, Attachment};

use crate::host::{hook::thunk::ThunkPool, ModHost};

mod detour;
mod host;

static INSTANCE: OnceLock<usize> = OnceLock::new();
/// https://learn.microsoft.com/en-us/windows/win32/dlls/dllmain#parameters
const DLL_PROCESS_ATTACH: u32 = 1;

dll_syringe::payload_procedure! {
    fn me_attach(request: AttachRequest) -> AttachResult {
        let host = ModHost::new(ThunkPool::new()?);
        host.attach();


        let host = ModHost::get_attached_mut();

        Ok(Attachment)
    }
}

#[no_mangle]
pub extern "stdcall" fn DllMain(instance: usize, reason: u32, _: *mut usize) -> i32 {
    if reason == DLL_PROCESS_ATTACH {
        let _ = INSTANCE.set(instance);
    }

    1
}
