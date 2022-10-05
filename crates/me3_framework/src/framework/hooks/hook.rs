#[macro_export]
macro_rules! hook {
    ($fn_ref:path, $callback:expr) => {{
        use $crate::FrameworkGlobal;
        unsafe {
            let hooks = $crate::hooks::Hooks::get_unchecked();
            hooks.install(&mut $fn_ref, $callback)
        }
    }};
}
