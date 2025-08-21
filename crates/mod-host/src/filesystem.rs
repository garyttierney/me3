use std::{
    ffi::OsString,
    mem,
    os::windows::{ffi::OsStringExt, raw::HANDLE},
    sync::Arc,
};

use eyre::OptionExt;
use me3_mod_host_assets::mapping::VfsOverrideMapping;
use tracing::{info, info_span, instrument};
use windows::{
    core::{s, w, BOOL, PCSTR, PCWSTR},
    Win32::{
        Foundation::HMODULE,
        Security::SECURITY_ATTRIBUTES,
        Storage::FileSystem::{
            CREATEFILE2_EXTENDED_PARAMETERS, FILE_CREATION_DISPOSITION, FILE_FLAGS_AND_ATTRIBUTES,
            FILE_SHARE_MODE,
        },
        System::LibraryLoader::{GetModuleHandleW, GetProcAddress},
    },
};

use crate::host::ModHost;

#[instrument(name = "filesystem", skip_all)]
pub fn attach_override(mapping: Arc<VfsOverrideMapping>) -> Result<(), eyre::Error> {
    let kernelbase = unsafe { GetModuleHandleW(w!("kernelbase.dll"))? };

    hook_create_file(kernelbase, mapping.clone())?;

    hook_create_directory(kernelbase, mapping.clone())?;

    hook_delete_file(kernelbase, mapping.clone())?;

    Ok(())
}

#[instrument(name = "create_file", skip_all)]
fn hook_create_file(kb: HMODULE, mapping: Arc<VfsOverrideMapping>) -> Result<(), eyre::Error> {
    type CreateFileA = unsafe extern "C" fn(
        lpfilename: PCSTR,
        dwdesiredaccess: u32,
        dwsharemode: FILE_SHARE_MODE,
        lpsecurityattributes: *const SECURITY_ATTRIBUTES,
        dwcreationdisposition: FILE_CREATION_DISPOSITION,
        dwflagsandattributes: FILE_FLAGS_AND_ATTRIBUTES,
        htemplatefile: HANDLE,
    ) -> HANDLE;

    type CreateFileW = unsafe extern "C" fn(
        lpfilename: PCWSTR,
        dwdesiredaccess: u32,
        dwsharemode: FILE_SHARE_MODE,
        lpsecurityattributes: *const SECURITY_ATTRIBUTES,
        dwcreationdisposition: FILE_CREATION_DISPOSITION,
        dwflagsandattributes: FILE_FLAGS_AND_ATTRIBUTES,
        htemplatefile: HANDLE,
    ) -> HANDLE;

    type CreateFile2 = unsafe extern "C" fn(
        lpfilename: PCWSTR,
        dwdesiredaccess: u32,
        dwsharemode: FILE_SHARE_MODE,
        dwcreationdisposition: FILE_CREATION_DISPOSITION,
        pcreateexparams: *const CREATEFILE2_EXTENDED_PARAMETERS,
    ) -> HANDLE;

    let (create_file_a, create_file_w, create_file_2) = unsafe {
        let create_file_a =
            GetProcAddress(kb, s!("CreateFileA")).ok_or_eyre("CreateFileA not found")?;
        let create_file_w =
            GetProcAddress(kb, s!("CreateFileW")).ok_or_eyre("CreateFileW not found")?;
        let create_file_2 =
            GetProcAddress(kb, s!("CreateFile2")).ok_or_eyre("CreateFile2 not found")?;

        (
            mem::transmute::<_, CreateFileA>(create_file_a),
            mem::transmute::<_, CreateFileW>(create_file_w),
            mem::transmute::<_, CreateFile2>(create_file_2),
        )
    };

    ModHost::get_attached()
        .hook(create_file_a)
        .with_span(info_span!("create_file_a"))
        .with_closure({
            let mapping = mapping.clone();

            move |p1, p2, p3, p4, p5, p6, p7, trampoline| unsafe {
                if p1.is_null() {
                    return trampoline(p1, p2, p3, p4, p5, p6, p7);
                }

                if let Ok(path) = p1.to_string()
                    && let Some(mapped_override) = mapping.disk_override(path)
                {
                    info!("override" = %mapped_override);

                    return trampoline(mapped_override.into(), p2, p3, p4, p5, p6, p7);
                }

                trampoline(p1, p2, p3, p4, p5, p6, p7)
            }
        })
        .install()?;

    ModHost::get_attached()
        .hook(create_file_w)
        .with_span(info_span!("create_file_w"))
        .with_closure({
            let mapping = mapping.clone();

            move |p1, p2, p3, p4, p5, p6, p7, trampoline| unsafe {
                if p1.is_null() {
                    return trampoline(p1, p2, p3, p4, p5, p6, p7);
                }

                let path = OsString::from_wide(p1.as_wide());

                if let Some(mapped_override) = mapping.disk_override(path) {
                    info!("override" = %mapped_override);

                    return trampoline(mapped_override.into(), p2, p3, p4, p5, p6, p7);
                }

                trampoline(p1, p2, p3, p4, p5, p6, p7)
            }
        })
        .install()?;

    ModHost::get_attached()
        .hook(create_file_2)
        .with_span(info_span!("create_file_2"))
        .with_closure({
            let mapping = mapping.clone();

            move |p1, p2, p3, p4, p5, trampoline| unsafe {
                if p1.is_null() {
                    return trampoline(p1, p2, p3, p4, p5);
                }

                let path = OsString::from_wide(p1.as_wide());

                if let Some(mapped_override) = mapping.disk_override(path) {
                    info!("override" = %mapped_override);

                    return trampoline(mapped_override.into(), p2, p3, p4, p5);
                }

                trampoline(p1, p2, p3, p4, p5)
            }
        })
        .install()?;

    info!("applied filesystem hook");

    Ok(())
}

#[instrument(name = "create_directory", skip_all)]
fn hook_create_directory(kb: HMODULE, mapping: Arc<VfsOverrideMapping>) -> Result<(), eyre::Error> {
    type CreateDirectoryA = unsafe extern "C" fn(
        lppathname: PCSTR,
        lpsecurityattributes: *const SECURITY_ATTRIBUTES,
    ) -> HANDLE;

    type CreateDirectoryW = unsafe extern "C" fn(
        lppathname: PCWSTR,
        lpsecurityattributes: *const SECURITY_ATTRIBUTES,
    ) -> HANDLE;

    type CreateDirectoryExW = unsafe extern "C" fn(
        lptemplatedirectory: PCWSTR,
        lpnewdirectory: PCWSTR,
        lpsecurityattributes: *const SECURITY_ATTRIBUTES,
    ) -> HANDLE;

    let (create_dir_a, create_dir_w, create_dir_exw) = unsafe {
        let create_dir_a =
            GetProcAddress(kb, s!("CreateDirectoryA")).ok_or_eyre("CreateDirectoryA not found")?;
        let create_dir_w =
            GetProcAddress(kb, s!("CreateDirectoryW")).ok_or_eyre("CreateDirectoryW not found")?;
        let create_dir_exw = GetProcAddress(kb, s!("CreateDirectoryExW"))
            .ok_or_eyre("CreateDirectoryExW not found")?;

        (
            mem::transmute::<_, CreateDirectoryA>(create_dir_a),
            mem::transmute::<_, CreateDirectoryW>(create_dir_w),
            mem::transmute::<_, CreateDirectoryExW>(create_dir_exw),
        )
    };

    ModHost::get_attached()
        .hook(create_dir_a)
        .with_closure({
            let mapping = mapping.clone();

            move |p1, p2, trampoline| unsafe {
                if p1.is_null() {
                    trampoline(p1, p2)
                } else if let Ok(path) = p1.to_string()
                    && let Some(mapped_override) = mapping.disk_override(path)
                {
                    trampoline(mapped_override.into(), p2)
                } else {
                    trampoline(p1, p2)
                }
            }
        })
        .install()?;

    ModHost::get_attached()
        .hook(create_dir_w)
        .with_closure({
            let mapping = mapping.clone();

            move |p1, p2, trampoline| unsafe {
                if p1.is_null() {
                    trampoline(p1, p2)
                } else if let Some(mapped_override) =
                    mapping.disk_override(OsString::from_wide(p1.as_wide()))
                {
                    trampoline(mapped_override.into(), p2)
                } else {
                    trampoline(p1, p2)
                }
            }
        })
        .install()?;

    ModHost::get_attached()
        .hook(create_dir_exw)
        .with_closure({
            let mapping = mapping.clone();

            move |p1, p2, p3, trampoline| unsafe {
                if p1.is_null() {
                    trampoline(p1, p2, p3)
                } else if let Some(mapped_override) =
                    mapping.disk_override(OsString::from_wide(p1.as_wide()))
                {
                    trampoline(mapped_override.into(), p2, p3)
                } else {
                    trampoline(p1, p2, p3)
                }
            }
        })
        .install()?;

    info!("applied filesystem hook");

    Ok(())
}

#[instrument(name = "delete_file", skip_all)]
fn hook_delete_file(kb: HMODULE, mapping: Arc<VfsOverrideMapping>) -> Result<(), eyre::Error> {
    type DeleteFileA = unsafe extern "C" fn(lpfilename: PCSTR) -> BOOL;
    type DeleteFileW = unsafe extern "C" fn(lpfilename: PCWSTR) -> BOOL;

    let (delete_file_a, delete_file_w) = unsafe {
        let delete_file_a =
            GetProcAddress(kb, s!("DeleteFileA")).ok_or_eyre("DeleteFileA not found")?;
        let delete_file_w =
            GetProcAddress(kb, s!("DeleteFileW")).ok_or_eyre("DeleteFileW not found")?;

        (
            mem::transmute::<_, DeleteFileA>(delete_file_a),
            mem::transmute::<_, DeleteFileW>(delete_file_w),
        )
    };

    ModHost::get_attached()
        .hook(delete_file_a)
        .with_closure({
            let mapping = mapping.clone();

            move |p1, trampoline| unsafe {
                if p1.is_null() {
                    trampoline(p1)
                } else if let Ok(path) = p1.to_string()
                    && let Some(mapped_override) = mapping.disk_override(path)
                {
                    trampoline(mapped_override.into())
                } else {
                    trampoline(p1)
                }
            }
        })
        .install()?;

    ModHost::get_attached()
        .hook(delete_file_w)
        .with_closure({
            let mapping = mapping.clone();

            move |p1, trampoline| unsafe {
                if p1.is_null() {
                    trampoline(p1)
                } else if let Some(mapped_override) =
                    mapping.disk_override(OsString::from_wide(p1.as_wide()))
                {
                    trampoline(mapped_override.into())
                } else {
                    trampoline(p1)
                }
            }
        })
        .install()?;

    info!("applied filesystem hook");

    Ok(())
}
