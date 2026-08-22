use std::{arch::asm, ptr::NonNull};

use closure_ffi::traits::FnPtr;
use diversion::installer::HookInstaller;
use pelite::pe::{Pe, Rva};

use crate::executable::Executable;

#[derive(Debug)]
pub struct IatHook<T> {
    inner: NonNull<T>,
}

/// Prepare an import address table hook by import name.
///
/// # Safety
///
/// The import is interpreted as being a function of type `T`. Getting its signature
/// wrong will cause UB when hooking the function. There are no strong atomicity guarantees.
pub unsafe fn install_iat_by_name<T>(
    exe: Executable,
    dll: &str,
    name: &str,
) -> pelite::Result<IatHook<T>>
where
    T: FnPtr + 'static,
{
    for import in exe.imports()? {
        if let Ok(import_dll) = import.dll_name()
            && dll.as_bytes().eq_ignore_ascii_case(&import_dll)
        {
            let desc = import.image();

            // Can't actually use methods provided by pelite because of the UB
            // unaligned references they unfortunately create.
            let ilt = exe.derva_slice_s(desc.OriginalFirstThunk, [0u8; 8])?;
            let iat = exe.derva_slice_s(desc.FirstThunk, [0u8; 8])?;

            for (import_name_rva, address) in ilt.iter().zip(iat) {
                let import_name_rva = usize::from_le_bytes(*import_name_rva);

                // Skip imports by ordinal (see MSDN PE32 docs).
                if import_name_rva >> (usize::BITS - 1) != 0 {
                    continue;
                }

                // There is a `u16` EAT ordinal hint before the name string.
                let Ok(import_name) = exe.derva_c_str(import_name_rva as Rva + 2) else {
                    continue;
                };

                if !name.as_bytes().eq_ignore_ascii_case(&import_name) {
                    continue;
                }

                // Can't ordinarily write to this pointer, see `update_thunk` below.
                return Ok(IatHook {
                    inner: NonNull::from_ref(address).cast::<T>(),
                });
            }

            break;
        }
    }

    // Failed to find the import.
    Err(pelite::Error::Null)
}

impl<T> HookInstaller for IatHook<T>
where
    T: FnPtr + 'static,
{
    type Target = T;
    type Context = ();

    fn target(&self) -> Self::Target {
        unsafe { self.inner.read_unaligned() }
    }

    fn update_thunk(&self, mut f: impl FnMut(Self::Target) -> Self::Target) -> Self::Target {
        unsafe {
            let old = self.inner.read_unaligned();
            let new = f(old);

            // Inline asm to take the write outside of the Rust AM (xchg is always atomic).
            asm!(
                "xchg [{}],{}",
                in(reg) self.inner.as_ptr(),
                in(reg) new.to_ptr(),
                options(nostack, preserves_flags)
            );

            old
        }
    }

    fn into_context(self) -> Self::Context {}
}

unsafe impl<T> Send for IatHook<T> {}

unsafe impl<T> Sync for IatHook<T> {}

#[allow(unused_macro_rules)]
macro_rules! install_iat {
    ($exe:expr, $dll:expr, $t:ident = $($def:tt)+) => {{
        #[allow(nonstandard_style)]
        type $t = unsafe extern "C" $($def)+;
        install_iat!($exe, $dll, $t)
    }};
    ($exe:expr, $dll:expr, $t:ty) => {
        <_ as ::eyre::WrapErr<_, _>>::wrap_err(
            $crate::hook::install_iat_by_name::<$t>($exe, $dll, stringify!($t)),
            concat!(stringify!($t), " not found"),
        )
    };
}

pub(crate) use install_iat;
