use std::ffi::c_char;

use eyre::ContextCompat;
use me3_mod_host_types::dlrf::RuntimeClassEntry;
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

    defer_init(Span::current(), Deferred::AfterPropsInit, move || {
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
