use std::{cell::OnceCell, mem, sync::Arc};

use closure_ffi::{
    traits::{FnPtr, FnThunk},
    BareFn,
};
use retour::Function;
use tracing::Span;

use crate::{
    detour::{install_detour, Detour, DetourError, UntypedDetour},
    host::append::{Append, WithAppended},
};

#[macro_export]
macro_rules! hook {
    {
        module = $module:expr,
        symbol = $symbol:literal,
        signature: $signature:ty,
        $closure:expr
    } => {{
            let fn_ptr = unsafe {
                let s = ::std::ffi::CString::new($symbol)?;
                ::windows::Win32::System::LibraryLoader::GetProcAddress(
                    $module,
                    ::windows::core::PCSTR::from_raw(s.as_ptr() as _)
                )
                .ok_or_eyre(concat!($symbol, " not found"))?
            };

            let fn_ptr = unsafe { ::std::mem::transmute::<_, $signature>(fn_ptr) };

            $crate::host::ModHost::get_attached()
                .hook(fn_ptr)
                .with_span(::tracing::info_span!($symbol))
                .with_closure($closure)
                .install()
    }};

    (
        pointer = $ptr:expr,
        signature = $signature:ty,
        $closure:expr
    ) => {{
        let fn_ptr = $ptr as $signature;

        $crate::host::ModHost::get_attached()
            .hook(fn_ptr)
            .with_closure($closure)
            .install()
    }};

    (
        pointer = $ptr:expr,
        $closure:expr
    ) => {{
        $crate::host::ModHost::get_attached()
            .hook($ptr)
            .with_closure($closure)
            .install()
    }};
}

pub enum HookSource<F: Function> {
    Function(F),
    Closure((F, &'static OnceCell<F>)),
}

pub struct HookInstaller<F>
where
    F: Function,
{
    enable_on_install: bool,
    on_install: Option<Box<dyn FnOnce(Arc<UntypedDetour>)>>,
    source: Option<HookSource<F>>,
    span: Span,
    target: F,
}

impl<F> HookInstaller<F>
where
    F: Function,
{
    pub fn new(target: F) -> Self {
        Self {
            enable_on_install: true,
            on_install: None,
            source: None,
            span: Span::none(),
            target,
        }
    }

    pub(crate) fn on_install<C>(self, c: C) -> Self
    where
        C: FnOnce(Arc<UntypedDetour>) + 'static,
    {
        Self {
            on_install: Some(Box::new(c)),
            ..self
        }
    }

    #[allow(unused)]
    pub fn with(&mut self, source: F) -> &mut Self {
        self.source = Some(HookSource::Function(source));
        self
    }

    pub fn with_closure<C>(&mut self, closure: C) -> &mut Self
    where
        C: Fn<<F::Arguments as Append<F>>::Output, Output = F::Output> + 'static,
        F: FnPtr,
        F::Arguments: Append<F>,
        (F::CC, WithAppended<C, F>): FnThunk<F>,
    {
        let span = mem::replace(&mut self.span, Span::none());

        let trampoline = Box::leak(Box::<OnceCell<F>>::default());

        let with_appended = WithAppended::new(closure, span, trampoline);

        let bare: BareFn<_> = with_appended.bare();

        self.source = Some(HookSource::Closure((bare.leak(), trampoline)));
        self
    }

    pub fn with_span(&mut self, span: Span) -> &mut Self {
        self.span = span;
        self
    }

    pub fn install(&mut self) -> Result<Arc<Detour<F>>, DetourError> {
        let mut uninit_trampoline = None;

        let hook = match self.source.take().expect("no hook source") {
            HookSource::Function(f) => f,
            HookSource::Closure((f, trampoline)) => {
                uninit_trampoline = Some(trampoline);
                f
            }
        };

        let detour = Arc::new(install_detour(self.target, hook)?);

        if let Some(on_install) = self.on_install.take() {
            on_install(unsafe { mem::transmute(detour.clone()) })
        }

        if let Some(trampoline) = uninit_trampoline {
            trampoline.get_or_init(|| detour.trampoline());
        }

        if self.enable_on_install {
            unsafe { detour.enable()? };
        }

        Ok(detour)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::HookInstaller;

    unsafe extern "C" fn test_fn() -> usize {
        5
    }

    #[test]
    fn context_with_closure() -> Result<(), Box<dyn Error>> {
        let hook = HookInstaller::<unsafe extern "C" fn() -> usize>::new(test_fn)
            .with_closure(|trampoline| 5 + unsafe { trampoline() })
            .install()?;

        unsafe { hook.enable()? };

        assert_eq!(10, unsafe { test_fn() });

        Ok(())
    }
}
