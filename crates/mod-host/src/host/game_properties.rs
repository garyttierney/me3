use std::{borrow::Cow, ffi::c_char, ptr::NonNull};

use eyre::ContextCompat;
use from_singleton::FromSingleton;
use me3_mod_host_types::{
    dlrf::RuntimeClassEntry,
    string::{custom::DlCustomUtf16Str, DlUtf16String},
    tree::{Tree, TreeMap},
};
use me3_mod_protocol::Game;
use rdvec::Vec as _;
use tracing::{instrument, Span};

use crate::{
    deferred::{defer_init, Deferred},
    host::ModHost,
};

pub fn start_offline() {
    ModHost::get_attached()
        .override_game_property("Menu.IsEnableOnlineMode", "false")
        .unwrap();
}

#[instrument(skip_all)]
pub fn attach_override(
    game: Game,
    runtime_classes: &[RuntimeClassEntry<'_>],
) -> Result<(), eyre::Error> {
    override_debug_properties(game, runtime_classes)?;

    override_system_properties(game)?;

    Ok(())
}

#[instrument(skip_all)]
fn override_debug_properties(
    game: Game,
    runtime_classes: &[RuntimeClassEntry<'_>],
) -> Result<(), eyre::Error> {
    let capi_name = if game < Game::EldenRing {
        "SprjAutoControlAPI"
    } else {
        "CSAutoControlAPI"
    };

    let capi_class = runtime_classes
        .iter()
        .find(|entry| entry.class.name == capi_name)
        .wrap_err_with(|| format!("failed to find runtime class for {capi_name}"))?
        .class;

    let set_game_prop_resolver = capi_class
        .methods
        .iter()
        .find(|m| m.name == "SetGameProperty")
        .wrap_err("SetGameProperty method not found")?
        .resolver;

    let set_game_prop_addr = set_game_prop_resolver
        .invokers
        .first()
        .wrap_err("SetGameProperty has no method invokers")?
        .addr;

    tracing::debug!(?set_game_prop_addr);

    let set_game_prop: unsafe extern "C" fn(*const c_char, *const c_char) =
        unsafe { std::mem::transmute(set_game_prop_addr) };

    defer_init(Span::current(), Deferred::AfterDbgPropsInit, move || {
        let overrides = ModHost::get_attached()
            .property_overrides
            .lock()
            .expect("poisoned");

        tracing::debug!("applying game property overrides (user has priority): {overrides:#?}");
        for (property, value) in overrides.internal.iter().chain(overrides.user.iter()) {
            unsafe { set_game_prop(property.as_ptr(), value.as_ptr()) }
        }
    })
}

#[instrument(skip_all)]
fn override_system_properties(game: Game) -> Result<(), eyre::Error> {
    defer_init(Span::current(), Deferred::AfterSysPropsInit, move || {
        let overrides = ModHost::get_attached()
            .property_overrides
            .lock()
            .expect("poisoned");

        let Some(mut system_properties) = (unsafe { PropertyMap::map_mut(game) }) else {
            tracing::error!("system property mapping is uninitialized or was not found");
            return;
        };

        tracing::debug!(
            "found system properties at {:016x}",
            system_properties.addr()
        );

        for (property, value) in overrides.internal.iter().chain(overrides.user.iter()) {
            // Property value pairs are sourced from Rust &str.
            system_properties.insert(property.to_str().unwrap(), value.to_str().unwrap());
        }
    })
}

#[repr(C)]
struct SystemProperties<T> {
    _vtable: usize,
    properties: NonNull<TreeMap<T, T>>,
}

#[repr(transparent)]
struct SprjSystemProperties(SystemProperties<DlUtf16String>);

#[repr(transparent)]
struct CSSystemProperties<T>(SystemProperties<T>);

enum PropertyMap<'a> {
    String(&'a mut dyn Tree<DlUtf16String, DlUtf16String>),
    Custom(&'a mut dyn Tree<DlCustomUtf16Str, DlCustomUtf16Str>),
}

impl<'a> PropertyMap<'a> {
    unsafe fn map_mut(game: Game) -> Option<PropertyMap<'a>> {
        if game < Game::EldenRing {
            unsafe {
                Some(PropertyMap::String(
                    from_singleton::address_of::<SprjSystemProperties>()?
                        .as_mut()
                        .0
                        .properties
                        .as_mut()
                        .as_mut_dyn(),
                ))
            }
        } else if game != Game::Nightreign {
            unsafe {
                Some(PropertyMap::String(
                    from_singleton::address_of::<CSSystemProperties<DlUtf16String>>()?
                        .as_mut()
                        .0
                        .properties
                        .as_mut()
                        .as_mut_dyn(),
                ))
            }
        } else {
            unsafe {
                Some(PropertyMap::Custom(
                    from_singleton::address_of::<CSSystemProperties<DlCustomUtf16Str>>()?
                        .as_mut()
                        .0
                        .properties
                        .as_mut()
                        .as_mut_dyn(),
                ))
            }
        }
    }

    fn insert(&mut self, property: &str, value: &str) {
        match self {
            Self::String(map) => _ = map.insert(property.into(), value.into()),
            Self::Custom(map) => _ = map.insert(property.into(), value.into()),
        }
    }

    fn addr(&self) -> usize {
        match self {
            Self::String(tree) => (&raw const *tree).addr(),
            Self::Custom(tree) => (&raw const *tree).addr(),
        }
    }
}

impl FromSingleton for SprjSystemProperties {}

impl<T> FromSingleton for CSSystemProperties<T> {
    fn name() -> std::borrow::Cow<'static, str> {
        Cow::Borrowed("CSSystemProperties")
    }
}
