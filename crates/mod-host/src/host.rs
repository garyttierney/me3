use std::{
    ffi::CString,
    fmt::Debug,
    marker::{FnPtr, Tuple},
    path::Path,
    sync::{Arc, OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard},
    time::Duration,
};

use libloading::{Library, Symbol};
use me3_mod_protocol::{native::NativeInitializerCondition, ModProfile};
use retour::Function;
use tracing::{error, info};

use self::hook::{thunk::ThunkPool, HookInstaller};
use crate::detour::UntypedDetour;

pub mod hook;

static ATTACHED_INSTANCE: OnceLock<RwLock<ModHost>> = OnceLock::new();

pub struct ModHost {
    hooks: Vec<Arc<UntypedDetour>>,
    native_modules: Vec<Library>,
    profiles: Vec<ModProfile>,
    thunk_pool: ThunkPool,
}

impl Debug for ModHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModHost")
            .field("hooks", &self.hooks)
            .field("profiles", &self.profiles)
            .field("thunk_pool", &self.thunk_pool)
            .finish()
    }
}

#[allow(unused)]
impl ModHost {
    pub fn new(thunk_pool: ThunkPool) -> Self {
        Self {
            hooks: vec![],
            native_modules: vec![],
            profiles: vec![],
            thunk_pool,
        }
    }

    pub fn load_native(
        &mut self,
        path: &Path,
        condition: Option<NativeInitializerCondition>,
    ) -> eyre::Result<()> {
        let module = unsafe { libloading::Library::new(path) }?;

        match condition {
            Some(NativeInitializerCondition::Delay { ms }) => {
                std::thread::sleep(Duration::from_millis(ms as u64))
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
            None => {}
        }

        self.native_modules.push(module);

        Ok(())
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

    pub fn hook<F>(&mut self, target: F) -> HookInstaller<F>
    where
        F: Function + FnPtr,
        F::Arguments: Tuple,
    {
        HookInstaller::new(Some(&mut self.hooks), &self.thunk_pool, target)
    }
}
