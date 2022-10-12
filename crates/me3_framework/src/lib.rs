#![feature(pointer_byte_offsets)]
#![feature(naked_functions)]
#![feature(core_intrinsics)]
#![allow(dead_code)]
#![feature(fn_traits, unboxed_closures)]
#![feature(strict_provenance)]

mod framework;

#[macro_export]
macro_rules! deref {
    ($name:ident) => { std::ptr::NonNull::new($name as *mut usize) };
    ($base:tt + $displacement:expr) => { deref!($base).map(|ptr| std::ptr::NonNull::new_unchecked(ptr.as_ptr().byte_add($displacement) as *mut usize)) };
    ([$( $body:tt )+]) => {
        deref!($($body)+).and_then(|ptr| std::ptr::NonNull::new(*ptr.as_ptr() as *mut usize))
    };
}

pub use faithe;
pub use faithe::{global, interface};
pub use framework::{hooks, overlay, scripting, tracing, vfs};
pub use framework::{Framework, FrameworkBuilder, FrameworkError, FrameworkGlobal};
