use std::{
    ffi::{CStr, CString, OsString},
    os::windows::{ffi::OsStringExt, raw::HANDLE},
    sync::Arc,
};

use closure_ffi::{thunk_factory, traits::FnPtr};
use diversion::{hook::leak::StaticHook, install};
use eyre::eyre;
use me3_mod_host_assets::mapping::{VfsOverride, VfsOverrideMapping};
use tracing::{info, info_span, instrument, Span};
use windows::{
    core::{w, BOOL, PCSTR, PCWSTR},
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

type CreateDirectoryA = unsafe extern "C" fn(
    lppathname: PCSTR,
    lpsecurityattributes: *const SECURITY_ATTRIBUTES,
) -> HANDLE;

type CreateDirectoryW = unsafe extern "C" fn(
    lppathname: PCWSTR,
    lpsecurityattributes: *const SECURITY_ATTRIBUTES,
) -> HANDLE;

type DeleteFileA = unsafe extern "C" fn(lpfilename: PCSTR) -> BOOL;

type DeleteFileW = unsafe extern "C" fn(lpfilename: PCWSTR) -> BOOL;

#[instrument(name = "filesystem", skip_all)]
pub fn attach_override(mapping: Arc<VfsOverrideMapping>) -> Result<(), eyre::Error> {
    unsafe {
        let kernelbase = GetModuleHandleW(w!("kernelbase.dll"))?;

        hook!(kernelbase, CreateFileA, mapping, args.0)?;
        hook!(kernelbase, CreateFileW, mapping, args.0)?;
        hook!(kernelbase, CreateFile2, mapping, args.0)?;

        hook!(kernelbase, CreateDirectoryA, mapping, args.0)?;
        hook!(kernelbase, CreateDirectoryW, mapping, args.0)?;

        hook!(kernelbase, DeleteFileA, mapping, args.0)?;
        hook!(kernelbase, DeleteFileW, mapping, args.0)?;
    }

    info!("applied filesystem hook");

    Ok(())
}

macro_rules! hook {
    ($module:ident, $name:ident, $mapping:ident, args.$argn:tt$(,)?) => {{
        static NAME: &str = stringify!($name);
        let span = info_span!(NAME);
        hook_impl::<$name, _>($module, NAME, $mapping.clone(), span, |args| {
            &mut args.$argn
        })
    }};
}

use hook;

unsafe fn hook_impl<T, S>(
    module: HMODULE,
    name: &str,
    mapping: Arc<VfsOverrideMapping>,
    span: Span,
    f: impl for<'a> Fn(&'a mut T::Args<'_, '_, '_>) -> &'a mut S + Send + Sync + 'static,
) -> eyre::Result<()>
where
    T: FnPtr + 'static,
    VfsOverrideMapping: VfsOverrideCStr<S>,
{
    let fn_ptr = unsafe {
        let c_str = CString::new(name).unwrap();
        let fn_proc = GetProcAddress(module, PCSTR(c_str.as_ptr() as _))
            .ok_or_else(|| eyre!("export {name} not found"))?;

        T::from_ptr(fn_proc as *const ())
    };

    unsafe {
        install(fn_ptr)?.static_hook_with_thunk(|hook| {
            thunk_factory::make_send_sync(move |mut args| {
                let _guard = span.enter();
                let c_str = f(&mut args);

                let mapped_override = mapping.c_str_override(c_str);
                if let Some(mapped_override) = &mapped_override {
                    *c_str = <VfsOverrideMapping as VfsOverrideCStr<S>>::override_as_c_str(
                        mapped_override,
                    );
                }

                hook.call_original(args)
            })
        });
    }

    Ok(())
}

trait VfsOverrideCStr<T> {
    type Override<'a>
    where
        Self: 'a;

    fn c_str_override<'a>(&'a self, c_str: &T) -> Option<Self::Override<'a>>;

    fn override_as_c_str(over: &Self::Override<'_>) -> T;
}

impl VfsOverrideCStr<PCSTR> for VfsOverrideMapping {
    type Override<'a> = Vec<u8>;

    fn c_str_override<'a>(&'a self, c_str: &PCSTR) -> Option<Self::Override<'a>> {
        if c_str.is_null() {
            return None;
        }

        let path = unsafe { CStr::from_ptr(c_str.as_ptr() as _).to_str().ok()? };
        let mapped_override = self.disk_or_uid_to_disk(path)?;

        info!("override" = %mapped_override);

        Some(mapped_override.to_c_string())
    }

    fn override_as_c_str(over: &Self::Override<'_>) -> PCSTR {
        PCSTR(over.as_ptr())
    }
}

impl VfsOverrideCStr<PCWSTR> for VfsOverrideMapping {
    type Override<'a> = &'a VfsOverride<'a>;

    fn c_str_override<'a>(&'a self, c_str: &PCWSTR) -> Option<Self::Override<'a>> {
        if c_str.is_null() {
            return None;
        }

        let path = unsafe { OsString::from_wide(c_str.as_wide()) };
        let mapped_override = self.disk_or_uid_to_disk(path)?;

        info!("override" = %mapped_override);

        Some(mapped_override)
    }

    fn override_as_c_str(over: &Self::Override<'_>) -> PCWSTR {
        (*over).into()
    }
}
