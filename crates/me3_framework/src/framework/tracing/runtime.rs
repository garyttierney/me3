use tracy_client::sys::{
    ___tracy_c_zone_context, ___tracy_emit_zone_begin_alloc, ___tracy_emit_zone_end,
};

extern "C" {
    pub fn profiler_entry();
    pub fn profiler_exit();
}

#[no_mangle]
pub extern "C" fn __profiler_begin(name: u64) -> ___tracy_c_zone_context {
    unsafe { ___tracy_emit_zone_begin_alloc(name, 1) }
}

#[no_mangle]
pub extern "C" fn __profiler_end(ctx: tracy_client::sys::___tracy_c_zone_context) {
    unsafe { ___tracy_emit_zone_end(ctx) }
}
