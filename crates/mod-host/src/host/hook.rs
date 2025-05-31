use std::{marker::Tuple, mem::MaybeUninit, sync::Arc};

use curried::Prepend;
use retour::Function;
use thunk::thunk_data;

use crate::{
    detour::{install_detour, Detour, DetourError, UntypedDetour},
    host::hook::thunk::ThunkPool,
};

pub mod curried;
pub mod thunk;

pub struct HookContext<F: Function> {
    pub trampoline: F,
}

pub enum HookSource<F: Function>
where
    F::Arguments: Tuple,
{
    FunctionPointer(F),
    Closure(Box<dyn Fn<F::Arguments, Output = F::Output>>),
}

pub struct HookInstaller<'a, F>
where
    F: Function,
    F::Arguments: Tuple,
{
    enable_on_install: bool,
    source: Option<HookSource<F>>,
    storage: Option<&'a mut Vec<Arc<UntypedDetour>>>,
    target: F,
    thunk_pool: &'a ThunkPool,
}

impl<'a, F> HookInstaller<'a, F>
where
    F: Function,
    F::Arguments: Tuple,
{
    pub fn new(
        storage: Option<&'a mut Vec<Arc<UntypedDetour>>>,
        thunk_pool: &'a ThunkPool,
        target: F,
    ) -> Self {
        Self {
            enable_on_install: true,
            source: None,
            storage,
            target,
            thunk_pool,
        }
    }

    #[allow(unused)]
    pub fn with(self, source: F) -> Self {
        Self {
            source: Some(HookSource::FunctionPointer(source)),
            ..self
        }
    }

    pub fn with_closure<C>(self, closure: C) -> Self
    where
        F::Arguments: Prepend<HookContext<F>>,
        C: Fn<<F::Arguments as Prepend<HookContext<F>>>::Output, Output = F::Output> + 'static,
    {
        let curried = curried::Curried::new(closure, || {
            let trampoline_ptr = unsafe { thunk_data::<F>().expect("no thunk data present") };
            let trampoline = unsafe { trampoline_ptr.read() };

            HookContext { trampoline }
        });

        Self {
            source: Some(HookSource::Closure(Box::new(curried))),
            ..self
        }
    }

    pub fn install(self) -> Result<Arc<Detour<F>>, DetourError> {
        let mut trampoline_ptr = None;
        let hook = match self.source.expect("no hook source") {
            HookSource::FunctionPointer(ptr) => ptr,
            HookSource::Closure(closure) => {
                let (thunk, thunk_trampoline_ptr) = self
                    .thunk_pool
                    .get::<F, _>(closure, MaybeUninit::<F>::uninit())
                    .expect("no free thunks available in pool");

                trampoline_ptr = Some(thunk_trampoline_ptr);
                thunk
            }
        };

        let detour = Arc::new(install_detour(self.target, hook)?);

        if let Some(storage) = self.storage {
            storage.push(unsafe { std::mem::transmute(detour.clone()) });
        }

        if let Some(mut trampoline_ptr) = trampoline_ptr {
            unsafe { trampoline_ptr.as_mut().write(detour.trampoline()) };
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

    use super::{thunk::ThunkPool, HookInstaller};

    type TestFn = extern "C" fn() -> usize;
    extern "C" fn test_fn() -> usize {
        5
    }

    #[test]
    fn context_with_closure() -> Result<(), Box<dyn Error>> {
        let thunks = ThunkPool::new()?;
        let hook = HookInstaller::<TestFn>::new(None, &thunks, test_fn)
            .with_closure(|ctx| 5 + (ctx.trampoline)())
            .install()?;

        unsafe { hook.enable()? };

        assert_eq!(10, test_fn());

        Ok(())
    }
}
