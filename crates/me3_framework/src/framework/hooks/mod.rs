use std::arch::asm;
use std::{mem, sync::RwLock};

use detour::{Function, RawDetour};
use dynasmrt::DynasmApi;
use dynasmrt::DynasmLabelApi;
use dynasmrt::ExecutableBuffer;
use dynasmrt::{dynasm, x64::Assembler};
use once_cell::sync::OnceCell;

pub use self::function_ref::{FunctionAddress, FunctionRef};
use crate::{FrameworkError, FrameworkGlobal};

mod function_ref;
mod hook;

pub struct Hook {
    buffer: ExecutableBuffer,
    detour: RawDetour,
}

#[derive(Default)]
pub struct Hooks {
    installed_hooks: RwLock<Vec<Hook>>,
}

impl FrameworkGlobal for Hooks {
    fn cell() -> &'static OnceCell<Self> {
        static INSTANCE: OnceCell<Hooks> = OnceCell::new();
        &INSTANCE
    }

    fn create() -> Result<Self, super::FrameworkError> {
        Ok(Hooks {
            installed_hooks: RwLock::new(vec![]),
        })
    }
}

impl Hooks {
    pub fn install<T, D>(&self, function: &T, detour: D) -> Result<(), FrameworkError>
    where
        T: FunctionRef,
        T::Target: Function,
        D: 'static
            + Fn<
                <<T as FunctionRef>::Target as Function>::Arguments,
                Output = <<T as FunctionRef>::Target as Function>::Output,
            >,
    {
        let closure_ptr = Box::into_raw(Box::new(Box::new(detour)
            as Box<
                dyn Fn<
                    <<T as FunctionRef>::Target as Function>::Arguments,
                    Output = <<T as FunctionRef>::Target as Function>::Output,
                >,
            >));

        let callback = Self::callback::<<<T as FunctionRef>::Target as Function>::Arguments, D>;
        let mut ops = Assembler::new().expect("unable to create assembler");

        let trampoline_offset = ops.offset();
        dynasm!(ops
            ; -> prelude:
            ; mov rax, QWORD closure_ptr as *const () as _
            ; mov r11, QWORD callback as *const () as _
            ; jmp r11
            ; int3
        );

        ops.commit()?;

        let buffer = ops.finalize().expect("unable to assemble hook trampoline");

        let trampoline = unsafe { mem::transmute(buffer.ptr(trampoline_offset)) };
        let detour = unsafe { RawDetour::new(function.get_ptr(), trampoline)? };

        function.set_target(detour.trampoline());
        unsafe { detour.enable()? };

        self.installed_hooks
            .write()
            .expect("unable to get write lock")
            .push(Hook { buffer, detour });

        Ok(())
    }

    #[naked]
    unsafe extern "C" fn get_trampoline_closure() -> *const () {
        asm!("ret", options(noreturn)) // trampoline already put the closure in RAX
    }

    unsafe extern "C" fn callback<A, F: Fn<A> + 'static>(args: A) -> F::Output {
        let closure = Self::get_trampoline_closure() as *const Box<F>;
        std::ops::Fn::call(&**closure, args)
    }
}

#[cfg(test)]
mod test {
    use std::cell::RefCell;

    use super::*;
    use crate::function;

    pub struct HookTarget {
        ptr: RefCell<*const ()>,
    }

    impl FunctionRef for HookTarget {
        type Target = extern "C" fn(v: i32) -> i32;

        fn set_target(&self, new_target: *const ()) {
            *self.ptr.borrow_mut() = new_target;
        }

        fn get_target(&self) -> Self::Target {
            unsafe { std::mem::transmute(self.get_ptr()) }
        }

        fn get_ptr(&self) -> *const () {
            *self.ptr.borrow()
        }
    }

    // Rust executes tests in parallel, so create a new function for every hook
    // to avoid detouring the same function multiple times.
    fn test_hook(func: fn(i32) -> i32) -> (Hooks, HookTarget) {
        let hooks = Hooks::default();
        let target = HookTarget {
            ptr: RefCell::new(func as *const _),
        };

        (hooks, target)
    }

    #[test]
    fn closure_calls_original_function() {
        function! {
            pub TEST_FN: extern "C" fn() -> i32 = "dummy.exe"#0xcafebabe;
        }

        let test_fn: fn() -> i32 = || 20;
        let hooks = Hooks::get_or_create().unwrap();
        unsafe { TEST_FN.set_target(test_fn as *const ()) };

        let new_value = 10;
        unsafe {
            hooks
                .install(&TEST_FN, move || new_value + TEST_FN.call())
                .expect("failed to install hook in perfect_closure_forwarding_ffi test");
        }

        assert_eq!(30, test_fn());
    }

    #[test]
    fn closure_captures_environment() {
        fn test_fn(v: i32) -> i32 {
            v * 2
        }
        let (hooks, target) = test_hook(test_fn);

        let new_value = 50;
        hooks
            .install(&target, move |_v| new_value)
            .expect("failed to install hook in perfect_closure_forwarding_ffi test");

        assert_eq!(new_value, test_fn(10));
    }

    #[test]
    fn closure_forwards_arguments() {
        fn test_fn(v: i32) -> i32 {
            v * 2
        }
        let (hooks, target) = test_hook(test_fn);

        hooks
            .install(&target, |v| v)
            .expect("failed to install hook in closure_forwards_arguments test");

        assert_eq!(42, test_fn(42));
    }

    #[test]
    fn hook_restored_on_drop() {
        fn test_fn(v: i32) -> i32 {
            v * 2
        }

        let (hooks, target) = test_hook(test_fn);

        hooks
            .install(&target, |v| v)
            .expect("failed to install hook in perfect_closure_forwarding_ffi test");

        assert_eq!(42, test_fn(42));

        std::mem::drop(hooks);

        assert_eq!(20, test_fn(10));
    }
}
