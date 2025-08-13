use std::{
    collections::HashMap,
    ffi::CString,
    fmt::Debug,
    marker::Tuple,
    panic,
    path::Path,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use closure_ffi::traits::FnPtr;
use libloading::{Library, Symbol};
use me3_launcher_attach_protocol::AttachConfig;
use me3_mod_protocol::{native::NativeInitializerCondition, Game, ModProfile};
use retour::Function;
use tracing::{error, info, warn, Span};

use self::hook::HookInstaller;
use crate::{
    detour::UntypedDetour,
    native::{ModEngineConnectorShim, ModEngineExtension, ModEngineInitializer},
};

mod append;
pub mod game_properties;
pub mod hook;

static ATTACHED_INSTANCE: OnceLock<ModHost> = OnceLock::new();

#[derive(Default)]
pub struct ModHost {
    hooks: Mutex<Vec<Arc<UntypedDetour>>>,
    native_modules: Mutex<Vec<Library>>,
    profiles: Vec<ModProfile>,
    property_overrides: Mutex<HashMap<Vec<u16>, bool>>,
}

impl Debug for ModHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModHost")
            .field("hooks", &self.hooks)
            .field("profiles", &self.profiles)
            .field("property_overrides", &self.property_overrides)
            .finish()
    }
}

#[allow(unused)]
impl ModHost {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_native(
        &self,
        path: &Path,
        condition: &Option<NativeInitializerCondition>,
    ) -> eyre::Result<()> {
        let result = panic::catch_unwind(|| {
            let module = unsafe { libloading::Library::new(path)? };

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

            Ok(module)
        });

        match result {
            Err(exception) => {
                warn!("an error occurred while loading {path:?}, it may not work as expected");
                Ok(())
            }
            Ok(result) => result.map(|module| {
                self.native_modules.lock().unwrap().push(module);
            }),
        }
    }

    pub fn get_attached() -> &'static ModHost {
        ATTACHED_INSTANCE.get().expect("not attached")
    }

    pub fn attach(self) {
        ATTACHED_INSTANCE.set(self).expect("already attached");
    }

    pub fn hook<F>(&'static self, target: F) -> HookInstaller<F>
    where
        F: Function + FnPtr,
        F::Arguments: Tuple,
    {
        HookInstaller::new(target).on_install(|hook| self.hooks.lock().unwrap().push(hook))
    }

    pub fn override_game_property<S: AsRef<str>>(&self, property: S, state: bool) {
        self.property_overrides
            .lock()
            .unwrap()
            .insert(property.as_ref().encode_utf16().collect(), state);
    }
}

pub fn dearxan(attach_config: &AttachConfig) {
    if !attach_config.disable_arxan && attach_config.game != Game::DarkSouls3 {
        return;
    }

    info!(
        "game" = %attach_config.game,
        "disable_arxan" = attach_config.disable_arxan,
        "will attempt to disable Arxan code protection",
    );

    let span = Span::current();
    unsafe {
        dearxan::disabler::neuter_arxan(move |_, detected_arxan| {
            let _span_guard = span.enter();
            info!(detected_arxan, "dearxan::disabler::neuter_arxan finished");
        });
    }
}
