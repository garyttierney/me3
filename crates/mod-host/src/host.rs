use std::{
    ffi::CString,
    fmt::Debug,
    panic,
    path::Path,
    sync::{Mutex, OnceLock},
    time::Duration,
};

use eyre::Context;
use indexmap::IndexMap;
use libloading::{Library, Symbol};
use me3_launcher_attach_protocol::AttachConfig;
use me3_mod_protocol::{native::NativeInitializerCondition, Game, ModProfile};
use tracing::{error, info, warn};

use crate::native::{ModEngineConnectorShim, ModEngineExtension, ModEngineInitializer};

pub mod game_properties;

static ATTACHED_INSTANCE: OnceLock<ModHost> = OnceLock::new();

#[derive(Default, Debug)]
struct PropertyOverrides {
    /// Property overrides specified by the user (via the attach config).
    user: IndexMap<CString, CString>,
    /// Property overrides defined internally by me3.
    internal: IndexMap<CString, CString>,
}

#[derive(Default)]
pub struct ModHost {
    native_modules: Mutex<Vec<Library>>,
    profiles: Vec<ModProfile>,
    property_overrides: Mutex<PropertyOverrides>,
    pub disable_arxan: bool,
}

impl Debug for ModHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModHost")
            .field("profiles", &self.profiles)
            .field("property_overrides", &self.property_overrides)
            .finish()
    }
}

#[allow(unused)]
impl ModHost {
    #[inline]
    pub fn new(attach_config: &AttachConfig) -> eyre::Result<Self> {
        // Unconditionally disable Arxan in Dark Souls 3.
        let disable_arxan = attach_config.disable_arxan || attach_config.game == Game::DarkSouls3;

        let mut property_overrides = PropertyOverrides::default();
        for (name, value) in &attach_config.property_overrides {
            let name = CString::new(name.as_str())
                .wrap_err_with(|| format!("nul byte in game property name: {:?}", name))?;
            let value = CString::new(value.as_str())
                .wrap_err_with(|| format!("nul byte in game property value: {:?}", value))?;
            property_overrides.user.insert(name, value);
        }

        Ok(Self {
            disable_arxan,
            property_overrides: Mutex::new(property_overrides),
            ..Default::default()
        })
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

    #[inline]
    pub fn get_attached() -> &'static ModHost {
        ATTACHED_INSTANCE.get().expect("not attached")
    }

    #[inline]
    pub fn attach(self) {
        ATTACHED_INSTANCE.set(self).expect("already attached");
    }

    pub fn override_game_property(
        &self,
        property: impl AsRef<str>,
        value: impl AsRef<str>,
    ) -> eyre::Result<()> {
        let property = CString::new(property.as_ref()).wrap_err("Nul byte in property key")?;
        let value = CString::new(value.as_ref()).wrap_err("Nul byte in property value")?;

        self.property_overrides
            .lock()
            .expect("poisoned")
            .internal
            .insert(property, value);

        Ok(())
    }
}
