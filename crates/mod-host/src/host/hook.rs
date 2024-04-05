use std::{marker::Tuple, mem::MaybeUninit};

use retour::{Function, GenericDetour};

use self::thunk::ThunkPool;

pub mod thunk;

pub trait HookInstaller {
    fn install<F>(&mut self, target: F, hook: impl Fn<F::Arguments, Output = F::Output>)
    where
        F: Function,
        F::Arguments: Tuple;
}

pub fn install_hook<F>(
    thunks: &ThunkPool,
    target: F,
    hook: impl Fn<F::Arguments, Output = F::Output>,
) where
    F: Function,
    F::Arguments: Tuple,
{
    let (thunk, mut trampoline_ptr) =
        thunks.get_with_data::<F, _>(hook, MaybeUninit::<F>::uninit());

    let detour = unsafe { GenericDetour::<F>::new(target, thunk).unwrap() };

    unsafe {
        trampoline_ptr
            .as_mut()
            .write(F::from_ptr(detour.trampoline()));

        detour.enable().expect("failed to enable detour");
    }

    std::mem::forget(detour);
}

#[cfg(test)]
mod test {
    use super::{install_hook, thunk::ThunkPool};

    extern "system" fn target_func() -> i32 {
        20
    }

    #[test]
    fn test1() {
        let thunks = ThunkPool::new().expect("creating thunk allocator");
        install_hook::<extern "system" fn() -> i32>(&thunks, target_func, || 42);

        assert_eq!(42, target_func());
    }
}
