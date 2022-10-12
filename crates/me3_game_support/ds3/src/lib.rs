use std::{ffi::OsString, os::windows::prelude::OsStringExt};

use me3_game_support_fromsoft::sprj::{ParamRepository, SprjGame};

me3_framework::global! {
    extern PARAM_REPOSITORY: ParamRepository = "DarkSoulsIII.exe"#0x4798118;
    extern NETWORK_LOG_CALLBACK: extern "C" fn(a: u32,  b: *const u16, c: *const ()) = "DarkSoulsIII.exe"#0x4930d30;
}

impl SprjGame for DarkSouls3 {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn enable_file_overrides(&self) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "DARK SOULS III"
    }

    fn param_repository(&self) -> &'static ParamRepository {
        unsafe { PARAM_REPOSITORY.get_ref() }
    }
}

extern "C" fn network_log(level: u32, ptr: *const u16, _unknown: *const ()) {
    if level != 1300000001 {
        let message = unsafe {
            let len = (0..).take_while(|&i| *ptr.offset(i) != 0).count();
            let slice = std::slice::from_raw_parts(ptr, len);

            OsString::from_wide(slice)
        };

        log::info!("{}: {}", level, message.to_string_lossy());
    }
}

pub struct DarkSouls3;

impl DarkSouls3 {
    pub fn enable_network_logger(&self) {
        unsafe {
            *NETWORK_LOG_CALLBACK.get_mut() = network_log;
        }
    }
}
