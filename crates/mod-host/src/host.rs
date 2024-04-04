use std::{mem::MaybeUninit, sync::OnceLock};

use me3_launcher_attach_protocol::AttachRequest;
use me3_mod_protocol::ModProfile;
use retour::GenericDetour;

use self::hook::{thunk::ThunkAllocator, HookInstaller};

pub mod hook;

static ATTACHED_INSTANCE: OnceLock<ModHost> = OnceLock::new();

#[derive(Debug)]
pub struct ModHost {
    profiles: Vec<ModProfile>,
    thunks: ThunkAllocator,
}

impl HookInstaller for ModHost {
    fn install<F>(&mut self, target: F, hook: impl Fn<F::Arguments, Output = F::Output>)
    where
        F: retour::Function,
        F::Arguments: std::marker::Tuple,
    {
        let (thunk, mut trampoline_ptr) = self
            .thunks
            .allocate_with_data::<F, _>(hook, MaybeUninit::<F>::uninit());

        let detour =
            unsafe { GenericDetour::<F>::new(target, thunk).expect("failed to create detour") };

        unsafe {
            trampoline_ptr
                .as_mut()
                .write(F::from_ptr(detour.trampoline()));

            detour.enable().expect("failed to enable detour");
        }
    }
}

impl ModHost {
    pub fn attach(request: AttachRequest) {
        let AttachRequest { profiles } = request;
        let host = ModHost {
            profiles,
            thunks: ThunkAllocator::new().expect("failed to create thunk allocator"),
        };

        ATTACHED_INSTANCE
            .set(host)
            .expect("attach called before detaching previous instance");
    }
}
