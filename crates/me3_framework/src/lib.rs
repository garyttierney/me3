#![feature(pointer_byte_offsets)]
#![feature(naked_functions)]
#![feature(core_intrinsics)]
#![allow(dead_code)]
#![feature(fn_traits, unboxed_closures)]
mod framework;

#[macro_export]
macro_rules! deref {
    ($name:ident) => { $name as *const usize };
    ($base:tt + $displacement:expr) => { deref!($base).byte_add($displacement) as *const usize };
    ($base:tt - $displacement:expr) => { deref!($base).byte_sub($displacement) as *const usize };
    ([$( $body:tt )+]) => {
        *deref!($($body)+) as *const usize
    };
}

pub use faithe;

pub enum FunctionAddress {
    Offset(RuntimeOffset),
    Pointer(*const ()),
}

pub trait FunctionRef {
    type Target;

    fn set_target(&self, new_target: *const ());
    fn get_target(&self) -> Self::Target;
    fn get_ptr(&self) -> *const ();
}

use faithe::RuntimeOffset;
pub use faithe::{global, interface};
pub use framework::{hooks, overlay, tracing, vfs};
pub use framework::{Framework, FrameworkBuilder, FrameworkError, FrameworkGlobal};
