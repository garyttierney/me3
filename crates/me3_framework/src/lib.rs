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
pub use faithe::{global, interface};
pub use framework::{hooks, overlay, scripting, tracing, vfs};
pub use framework::{Framework, FrameworkBuilder, FrameworkError, FrameworkGlobal};
