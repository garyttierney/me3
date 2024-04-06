use std::{
    marker::{FnPtr, Tuple},
    sync::{Arc, OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use me3_mod_protocol::ModProfile;
use retour::Function;

use self::hook::{thunk::ThunkPool, HookInstaller};
use crate::detour::UntypedDetour;

pub mod hook;

static ATTACHED_INSTANCE: OnceLock<RwLock<ModHost>> = OnceLock::new();

#[derive(Debug)]
pub struct ModHost {
    hooks: Vec<Arc<UntypedDetour>>,
    profiles: Vec<ModProfile>,
    thunk_pool: ThunkPool,
}

impl ModHost {
    pub fn new(thunk_pool: ThunkPool) -> Self {
        Self {
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

    pub fn hook<F>(&mut self, target: F) -> HookInstaller<F>
    where
        F: Function + FnPtr,
        F::Arguments: Tuple,
    {
        HookInstaller::new(Some(&mut self.hooks), &self.thunk_pool, target)
    }
}
