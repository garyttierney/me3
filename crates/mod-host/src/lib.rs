#![feature(fn_traits)]
#![feature(fn_ptr_trait)]
#![feature(tuple_trait)]
#![feature(unboxed_closures)]
#![feature(naked_functions)]

use std::{collections::HashMap, sync::OnceLock};

use me3_launcher_attach_protocol::{AttachError, AttachRequest, AttachResult, Attachment};
use me3_mod_host_assets::mapping::ArchiveOverrideMapping;
use crate::host::{hook::thunk::ThunkPool, ModHost};

mod detour;
mod host;
mod asset_archive;

static INSTANCE: OnceLock<usize> = OnceLock::new();
/// https://learn.microsoft.com/en-us/windows/win32/dlls/dllmain#parameters
const DLL_PROCESS_ATTACH: u32 = 1;

dll_syringe::payload_procedure! {
    fn me_attach(request: AttachRequest) -> AttachResult {
        let mut host = ModHost::new(ThunkPool::new()?);

        let mut override_mapping = ArchiveOverrideMapping::default();
        override_mapping.scan_directories(request.packages.iter())
            .map_err(|e| AttachError("Failed to scan asset folder. {e:?}".to_string()))?;
        asset_archive::attach(&mut host, override_mapping);

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
