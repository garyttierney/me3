use std::{
    collections::HashMap,
    fmt::Debug,
    marker::Tuple,
    sync::{Arc, Mutex, OnceLock},
};

use bevy_ecs::{system::Res, world::Mut};
use closure_ffi::traits::FnPtr;
use me3_launcher_attach_protocol::AttachConfig;
use me3_mod_protocol::Game;
use retour::Function;
use tracing::{info, Span};

use self::hook::HookInstaller;
use crate::{
    app::{ExternalResource, Me3App},
    detour::UntypedDetour,
    plugins::properties::GameProperties,
};

mod append;

#[macro_use]
pub mod hook;

static ATTACHED_INSTANCE: OnceLock<ModHost> = OnceLock::new();

pub struct ModHost {
    pub(crate) app: Mutex<Me3App>,
    hooks: Mutex<Vec<Arc<UntypedDetour>>>,
    property_overrides: Mutex<HashMap<Vec<u16>, bool>>,
}

impl Debug for ModHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModHost")
            .field("hooks", &self.hooks)
            .field("property_overrides", &self.property_overrides)
            .finish()
    }
}

#[allow(unused)]
impl ModHost {
    pub fn new(app: Me3App) -> Self {
        Self {
            app: Mutex::new(app),
            hooks: Default::default(),
            property_overrides: Default::default(),
        }
    }

    pub fn with_app<R>(f: impl FnOnce(&ModHost, &mut Me3App) -> R) -> R {
        let attached = Self::get_attached();
        let mut app = attached.app.lock().expect("failed to lock app");

        f(attached, &mut app)
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
        let mut app = self.app.lock().expect("failed to lock app");

        app.resource_scope(|world, props: Mut<GameProperties>| {
            props
                .lock()
                .unwrap()
                .insert(property.as_ref().encode_utf16().collect(), state)
        });
    }
}

pub fn dearxan(attach_config: Res<ExternalResource<AttachConfig>>) {
    if !attach_config.disable_arxan && attach_config.game != Game::DarkSouls3 {
        return;
    }

    info!(
        "game" = %attach_config.game,
        "attach_config.disable_arxan" = attach_config.disable_arxan,
        "will attempt to disable Arxan code protection",
    );

    let span = Span::current();
    unsafe {
        dearxan::disabler::neuter_arxan(move |result| {
            let _span_guard = span.enter();
            info!(?result, "dearxan::disabler::neuter_arxan finished");
        });
    }
}
