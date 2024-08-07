use std::{cell::OnceCell, io::Write, sync::Arc};

use me3_mod_host_assets::{ffi, hook::RSResourceFileRequest, mapping::AssetMapping};
use me3_mod_protocol::package::Package;
use retour::Function;
use thiserror::Error;

use crate::{
    detour::{install_detour, Detour, DetourError},
    host::ModHost,
};

pub type OpenHookFn = extern "C" fn(*mut RSResourceFileRequest) -> bool;

#[derive(Debug, Default)]
pub struct AssetLoadHook {
    mapping: Arc<AssetMapping>,
}

impl AssetLoadHook {
    pub fn new(mapping: AssetMapping) -> Self {
        Self {
            mapping: Arc::new(mapping),
        }
    }

    /// Attaches the asset load hook to a mod host
    pub fn attach(&mut self, host: &mut ModHost) -> Result<(), DetourError> {
        let hook_instance: Arc<OnceCell<Arc<Detour<OpenHookFn>>>> = Default::default();

        let hook = {
            let hook_instance = hook_instance.clone();
            let mapping = self.mapping.clone();

            host.hook(self.get_hook_location())
                .with_closure(move |request: *mut RSResourceFileRequest| -> bool {
                    let resource_path = unsafe { &request.as_ref().unwrap().resource_path };
                    let resource_path_string = ffi::get_dlwstring_contents(resource_path);

                    if let Some(mapped_override) = mapping.get_override(&resource_path_string) {
                        ffi::set_dlwstring_contents(resource_path, mapped_override);
                    }

                    hook_instance.get().unwrap().trampoline()(request)
                })
                .install()?
        };

        hook_instance.set(hook);

        Ok(())
    }

    // TODO: call into AssetHookLocationProvider trait and either AOB or do
    // vftable lookups depending on the game?
    fn get_hook_location(&self) -> OpenHookFn {
        unsafe { std::mem::transmute::<usize, OpenHookFn>(0x140128730usize) }
    }
}
