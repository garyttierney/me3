use bevy_app::PostStartup;
use bevy_ecs::{
    resource::Resource,
    schedule::{Schedule, ScheduleLabel},
    system::ResMut,
};
use libloading::{Library, Symbol};
use me3_launcher_attach_protocol::AttachConfig;
use me3_mod_protocol::native::{Native, NativeInitializerCondition};
use tracing::{error, info, warn};

use crate::{app::ExternalRes, host::ModHost, plugins::Plugin};

pub struct NativesPlugin;

use std::{
    ffi::{c_char, CString},
    panic,
    time::Duration,
};

pub type ModEngineInitializer =
    unsafe extern "C" fn(&ModEngineConnectorShim, &mut *mut ModEngineExtension) -> bool;

pub struct ModEngineConnectorShim;

pub struct ModEngineExtension {
    _destructor: extern "C" fn(),
    _on_attach: extern "C" fn(),
    _on_detach: extern "C" fn(),
    _id: extern "C" fn() -> *const c_char,
}

#[derive(Default, Resource)]
pub struct NativesCollection {
    loaded: Vec<Library>,
    delayed: Vec<Native>,
}

#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub struct LoadDelayedNatives;

impl NativesCollection {
    pub fn load(&mut self, native: &Native) -> color_eyre::Result<()> {
        let path = native.path.as_path();
        let result = panic::catch_unwind(|| {
            let module = unsafe { libloading::Library::new(path)? };

            match &native.initializer {
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
            Err(_) => {
                warn!("an error occurred while loading {path:?}, it may not work as expected");
                Ok(())
            }
            Ok(result) => result.map(|module| {
                self.loaded.push(module);
            }),
        }
    }
}

impl Plugin for NativesPlugin {
    fn build(&self, app: &mut crate::app::Me3App) {
        let delayed_native_schedule = Schedule::new(LoadDelayedNatives);
        app.add_schedule(delayed_native_schedule);

        app.init_resource::<NativesCollection>();
        app.register_system(PostStartup, Self::load_natives);
        app.register_system(LoadDelayedNatives, Self::load_delayed_natives);
    }
}

impl NativesPlugin {
    pub fn load_delayed_natives(mut natives: ResMut<NativesCollection>) -> bevy_ecs::error::Result {
        let delayed: Vec<_> = natives.delayed.drain(..).collect();
        for native in delayed {
            if let Err(e) = natives.load(&native) {
                warn!(
                    error = &*e,
                    path = %native.path.display(),
                    "failed to load native mod",
                );

                if !native.optional {
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    pub fn load_natives(
        config: ExternalRes<AttachConfig>,
        mut natives: ResMut<NativesCollection>,
    ) -> bevy_ecs::error::Result {
        let first_delayed_offset = config
            .natives
            .iter()
            .enumerate()
            .filter_map(|(idx, native)| native.initializer.is_some().then_some(idx))
            .next()
            .unwrap_or(config.natives.len());

        let (immediate, delayed) = config.natives.split_at(first_delayed_offset);

        for native in immediate {
            if let Err(e) = natives.load(native) {
                warn!(
                    error = &*e,
                    path = %native.path.display(),
                    "failed to load native mod",
                );

                if !native.optional {
                    return Err(e.into());
                }
            }
        }

        natives.delayed.extend(delayed.iter().cloned());

        std::thread::spawn(move || {
            ModHost::with_app(|_, app| {
                app.run_schedule(LoadDelayedNatives);
            })
        });

        Ok(())
    }
}

// asset_hooks::attach_override(
//     &**attach_config,
//     &**exe,
//     class_map,
//     &step_tables,
//     &**override_mapping,
// )
// .map_err(|e| {
//     e.wrap_err("failed to attach asset override hooks; no files will be overridden")
// })?;
