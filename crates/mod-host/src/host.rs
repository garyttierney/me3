use std::{
    ffi::CString,
    fmt::Debug,
    marker::Tuple,
    path::Path,
    sync::{Arc, OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard},
    time::Duration,
};

use closure_ffi::traits::FnPtr;
use libloading::{Library, Symbol};
use me3_mod_protocol::{native::NativeInitializerCondition, ModProfile};
use retour::Function;
use tracing::{error, info, warn};

use self::hook::HookInstaller;
use crate::{
    detour::UntypedDetour,
    native::{ModEngineConnectorShim, ModEngineExtension, ModEngineInitializer},
};

mod append;
pub mod hook;

static ATTACHED_INSTANCE: OnceLock<RwLock<ModHost>> = OnceLock::new();

#[derive(Default)]
pub struct ModHost {
    hooks: Vec<Arc<UntypedDetour>>,
    native_modules: Vec<Library>,
    profiles: Vec<ModProfile>,
}

impl Debug for ModHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModHost")
            .field("hooks", &self.hooks)
            .field("profiles", &self.profiles)
            .finish()
    }
}

#[allow(unused)]
impl ModHost {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_native(
        &mut self,
        path: &Path,
        condition: Option<NativeInitializerCondition>,
    ) -> eyre::Result<()> {
        let result = microseh::try_seh(|| {
            let module: Library = unsafe { libloading::Library::new(path) }?;

            match &condition {
                Some(NativeInitializerCondition::Delay { ms }) => {
                    std::thread::sleep(Duration::from_millis(*ms as u64))
                }
                Some(NativeInitializerCondition::Function(symbol)) => unsafe {
                    let sym_name = CString::new(symbol.as_bytes())?;
                    let initializer: Symbol<unsafe extern "C" fn() -> bool> =
                        module.get(sym_name.as_bytes_with_nul())?;

                    if initializer() {
                        info!(?path, symbol, "native initialized successfully");
                    } else {
                        error!(?path, symbol, "native failed to initialize");
                    }
                },
                None => {
                    let me2_initializer: Option<Symbol<ModEngineInitializer>> =
                        unsafe { module.get(b"modengine_ext_init\0").ok() };

                    let mut extension_ptr: *mut ModEngineExtension = std::ptr::null_mut();
                    if let Some(initializer) = me2_initializer {
                        unsafe { initializer(&ModEngineConnectorShim, &mut extension_ptr) };

                        info!(?path, "loaded native with me2 compatibility shim");
                    }
                }
            }

            self.native_modules.push(module);

            eyre::Ok(())
        });

        match result {
            Err(exception) => {
                warn!("an error occurred while loading {path:?}, it may not work as expected");
                Ok(())
            }
            Ok(result) => result,
        }
    }

    pub fn get_attached() -> RwLockReadGuard<'static, ModHost> {
        let lock = ATTACHED_INSTANCE.get().expect("not attached");

        match lock.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    pub fn get_attached_mut() -> RwLockWriteGuard<'static, ModHost> {
        let lock = ATTACHED_INSTANCE.get().expect("not attached");

        match lock.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    pub fn attach(self) {
        ATTACHED_INSTANCE
            .set(RwLock::new(self))
            .expect("already attached");
    }

    pub fn hook<F>(&mut self, target: F) -> HookInstaller<'_, F>
    where
        F: Function + FnPtr,
        F::Arguments: Tuple,
    {
        HookInstaller::new(Some(&mut self.hooks), target)
    }
}
