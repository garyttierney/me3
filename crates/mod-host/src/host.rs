use std::{
    collections::HashMap,
    ffi::CString,
    fmt::Debug,
    marker::Tuple,
    panic,
    path::Path,
    ptr,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use closure_ffi::traits::FnPtr;
use libloading::{Library, Symbol};
use me3_binary_analysis::pe;
use me3_launcher_attach_protocol::AttachConfig;
use me3_mod_protocol::{native::NativeInitializerCondition, Game, ModProfile};
use pelite::pe::Pe;
use regex::bytes::Regex;
use retour::Function;
use tracing::{error, info, warn, Span};
use windows::core::w;

use self::hook::HookInstaller;
use crate::{
    detour::UntypedDetour,
    executable::Executable,
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

pub fn dearxan(attach_config: &AttachConfig, executable: Executable) {
    if !attach_config.disable_arxan && attach_config.game != Game::DarkSouls3 {
        return;
    }

    info!(
        "game" = %attach_config.game,
        "attach_config.disable_arxan" = attach_config.disable_arxan,
        "will attempt to disable Arxan code protection",
    );

    let game = attach_config.game;
    let span = Span::current();
    unsafe {
        dearxan::disabler::neuter_arxan(move |result| {
            let _span_guard = span.enter();
            info!(?result, "dearxan::disabler::neuter_arxan finished");

            // Temporary patch for a data encryption method not handled by dearxan.
            // FIXME: remove when dearxan (currently 0.4.1) is updated.
            if game == Game::DarkSouls3 {
                let Ok(text_section) = pe::section(executable, ".text") else {
                    return;
                };

                let Ok(text) = executable.get_section_bytes(text_section) else {
                    return;
                };

                let re = Regex::new(
                    r"(?s-u)\x41\xb8\x1f\x00\x00\x00\x48\x8d\x15(.{4})[\x48|\x49]\x8b[\xc8-\xcf]\xe8.{4}",
                )
                .unwrap();

                let Some((_, [disp32 @ &[b0, b1, b2, b3]])) =
                    re.captures(text).map(|c| c.extract())
                else {
                    return;
                };

                let data_ptr = disp32
                    .as_ptr_range()
                    .end
                    .byte_offset(i32::from_le_bytes([b0, b1, b2, b3]) as _);

                ptr::copy_nonoverlapping(
                    w!("FDPrVuT4fAFvdHJYAgyMzRF4EcBAnKg").as_ptr(),
                    data_ptr as *mut u16,
                    32,
                );

                info!("applied dearxan patch for Dark Souls 3");
            }
        });
    }
}
