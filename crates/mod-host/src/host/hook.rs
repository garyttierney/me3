use std::{cell::OnceCell, mem, sync::Arc};

use closure_ffi::{
    traits::{FnPtr, FnThunk},
    BareFn,
};
use retour::Function;

use crate::{
    detour::{install_detour, Detour, DetourError, UntypedDetour},
    host::append::{Append, WithAppended},
};

pub enum HookSource<F: Function> {
    Function(F),
    Closure((F, &'static OnceCell<F>)),
}

pub struct HookInstaller<'a, F>
where
    F: Function,
{
    enable_on_install: bool,
    source: Option<HookSource<F>>,
    storage: Option<&'a mut Vec<Arc<UntypedDetour>>>,
    target: F,
}

impl<'a, F> HookInstaller<'a, F>
where
    F: Function,
{
    pub fn new(storage: Option<&'a mut Vec<Arc<UntypedDetour>>>, target: F) -> Self {
        Self {
            enable_on_install: true,
            source: None,
            storage,
            target,
        }
    }

    #[allow(unused)]
    pub fn with(self, source: F) -> Self {
        Self {
            source: Some(HookSource::Function(source)),
            ..self
        }
    }

    pub fn with_closure<C>(self, closure: C) -> Self
    where
        C: Fn<<F::Arguments as Append<F>>::Output, Output = F::Output> + 'static,
        F: FnPtr,
        F::Arguments: Append<F>,
        (F::CC, WithAppended<C, F>): FnThunk<F>,
    {
        let trampoline: &OnceCell<F> = Box::leak(Box::<OnceCell<F>>::default());

        let with_appended = WithAppended::new(closure, trampoline);

        let bare: BareFn<_> = with_appended.bare();

        Self {
            source: Some(HookSource::Closure((bare.leak(), trampoline))),
            ..self
        }
    }

    pub fn install(self) -> Result<Arc<Detour<F>>, DetourError> {
        let mut uninit_trampoline = None;

        let hook = match self.source.expect("no hook source") {
            HookSource::Function(f) => f,
            HookSource::Closure((f, trampoline)) => {
                uninit_trampoline = Some(trampoline);
                f
            }
        };

        let detour = Arc::new(install_detour(self.target, hook)?);

        if let Some(storage) = self.storage {
            storage.push(unsafe { mem::transmute(detour.clone()) });
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
        let hook_installer = HookInstaller::<unsafe extern "C" fn() -> usize>::new(None, test_fn);

        let hook = hook_installer
            .with_closure(|trampoline| 5 + unsafe { trampoline() })
            .install()?;

        unsafe { hook.enable()? };

        assert_eq!(10, unsafe { test_fn() });

        Ok(())
    }
}
