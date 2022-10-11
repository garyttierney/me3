use std::{mem::transmute, os::windows::raw::HANDLE};

use detour::GenericDetour;
use faithe::types::HINSTANCE;
use windows::{
    core::PCWSTR,
    Win32::{
        Security::SECURITY_ATTRIBUTES,
        Storage::FileSystem::{
            FILE_ACCESS_FLAGS, FILE_CREATION_DISPOSITION, FILE_FLAGS_AND_ATTRIBUTES,
            FILE_SHARE_MODE,
        },
        System::LibraryLoader::{GetModuleHandleA, GetProcAddress},
    },
};

use crate::framework::FrameworkError;

type FnCreateFileW = fn(
    lpfilename: PCWSTR,
    dwdesiredaccess: FILE_ACCESS_FLAGS,
    dwsharemode: FILE_SHARE_MODE,
    lpsecurityattributes: *const SECURITY_ATTRIBUTES,
    dwcreationdisposition: FILE_CREATION_DISPOSITION,
    dwflagsandattributes: FILE_FLAGS_AND_ATTRIBUTES,
    htemplatefile: HANDLE,
) -> HANDLE;

static mut O_CREATE_FILE: Option<GenericDetour<FnCreateFileW>> = None;

fn hk_create_file(
    lpfilename: PCWSTR,
    dwdesiredaccess: FILE_ACCESS_FLAGS,
    dwsharemode: FILE_SHARE_MODE,
    lpsecurityattributes: *const SECURITY_ATTRIBUTES,
    dwcreationdisposition: FILE_CREATION_DISPOSITION,
    dwflagsandattributes: FILE_FLAGS_AND_ATTRIBUTES,
    htemplatefile: HANDLE,
) -> HANDLE {
    unsafe {
        return O_CREATE_FILE.as_ref().unwrap().call(
            lpfilename,
            dwdesiredaccess,
            dwsharemode,
            lpsecurityattributes,
            dwcreationdisposition,
            dwflagsandattributes,
            htemplatefile,
        );
    }
}

#[allow(clippy::or_fun_call)] // false positive
fn get_proc_addr(
    module: HINSTANCE,
    name: &'static str,
) -> Result<unsafe extern "system" fn() -> isize, FrameworkError> {
    unsafe { GetProcAddress(module, name).ok_or(FrameworkError::NoSymbolFound(name)) }
}

pub(super) fn install_vfs_hooks() -> Result<(), FrameworkError> {
    let kernel32 = unsafe { GetModuleHandleA("kernel32.dll").expect("kernel32 not loaded") };

    let create_file_addr =
        unsafe { transmute::<_, FnCreateFileW>(get_proc_addr(kernel32, "CreateFileW")?) };

    let hook = unsafe {
        GenericDetour::<FnCreateFileW>::new(create_file_addr, hk_create_file)
            .expect("couldn't create detour")
    };

    unsafe {
        hook.enable().unwrap();
        O_CREATE_FILE = Some(hook);
    }

    Ok(())
}
