use std::marker::PhantomData;

use retour::{Function, RawDetour};

pub type UntypedDetour = Detour<fn() -> ()>;

#[derive(Debug)]
pub struct Detour<F: Function> {
    detour: RawDetour,
    ty: PhantomData<F>,
}

impl<F: Function> Detour<F> {
    pub unsafe fn disable(&self) -> Result<(), DetourError> {
        self.detour.disable()?;

        Ok(())
    }

    pub unsafe fn enable(&self) -> Result<(), DetourError> {
        self.detour.enable()?;

        Ok(())
    }

    pub fn trampoline(&self) -> F {
        unsafe { F::from_ptr(self.detour.trampoline() as *const _) }
    }
}

impl<F: Function> Drop for Detour<F> {
    fn drop(&mut self) {
        unsafe {
            let _ = self.disable();
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DetourError {
    #[error("Given target or hook is invalid as a detour")]
    InvalidDetour(#[from] retour::Error),
}

pub fn install_detour<F: Function>(target: F, hook: F) -> Result<Detour<F>, DetourError> {
    // SAFETY: both pointers are guaranteed to point to a function with the same type.
    let detour = unsafe { RawDetour::new(target.to_ptr(), hook.to_ptr())? };
    let handle = Detour {
        detour,
        ty: PhantomData,
    };

    Ok(handle)
}

#[cfg(test)]
mod test {
    use crate::detour::install_detour;

    extern "system" fn target_func() -> i32 {
        20
    }

    extern "system" fn overriden_target_func() -> i32 {
        42
    }

    #[test]
    fn test1() {
        let detour =
            install_detour::<extern "system" fn() -> i32>(target_func, overriden_target_func)
                .expect("failed to install");

        assert_eq!(20, target_func());

        unsafe { detour.enable().expect("failed to enable") };

        assert_eq!(42, target_func());
        assert_eq!(20, detour.trampoline()());

        unsafe { detour.disable().expect("failed to disable") };

        assert_eq!(20, target_func());
    }
}
