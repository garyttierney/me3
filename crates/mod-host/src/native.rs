use std::ffi::c_char;

pub type ModEngineInitializer =
    unsafe extern "C" fn(&ModEngineConnectorShim, &mut *mut ModEngineExtension) -> bool;

pub struct ModEngineConnectorShim;

pub struct ModEngineExtension {
    _destructor: extern "C" fn(),
    _on_attach: extern "C" fn(),
    _on_detach: extern "C" fn(),
    _id: extern "C" fn() -> *const c_char,
}
