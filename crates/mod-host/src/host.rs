use std::{
    fmt::Debug,
    marker::{FnPtr, Tuple},
    sync::{Arc, OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use crash_handler::CrashHandler;
use me3_mod_protocol::ModProfile;
use retour::Function;

use self::hook::{thunk::ThunkPool, HookInstaller};
use crate::detour::UntypedDetour;

pub mod hook;

static ATTACHED_INSTANCE: OnceLock<RwLock<ModHost>> = OnceLock::new();

pub struct ModHost {
    crash_handler: CrashHandler,
    hooks: Vec<Arc<UntypedDetour>>,
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
    pub fn new(crash_handler: CrashHandler, thunk_pool: ThunkPool) -> Self {
        Self {
            crash_handler: CrashHandler,
            hooks: vec![],
            profiles: vec![],
            thunk_pool,
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

    pub fn panic(&self) {
        let _ = self.crash_handler.simulate_exception(None);
    }

    pub fn hook<F>(&mut self, target: F) -> HookInstaller<F>
    where
        F: Function + FnPtr,
        F::Arguments: Tuple,
    {
        HookInstaller::new(Some(&mut self.hooks), &self.thunk_pool, target)
    }
}
