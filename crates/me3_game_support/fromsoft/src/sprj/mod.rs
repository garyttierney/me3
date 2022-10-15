pub mod formats;
mod param;

use std::any::Any;

pub use param::{ParamFileDescriptor, ParamRepository};

/// Global game instance for a FROM SOFTWARE game. This interface is exclusively
/// for code shared among the majority of titles supported by the "SPRJ" framework.
///
/// Game specific data/functions can be accessed by downcasting a [SprjGame] via [get_game_api]:
///
/// ```ignore
/// let game: &dyn SprjGame = &DarkSouls3;
/// if let Some(ds3) = get_game_api::<DarkSouls3>(game) {
///     ds3.enable_network_logging();
/// }
/// ```
pub trait SprjGame: Send + Sync {
    #[doc(hidden)]
    fn as_any(&self) -> &dyn Any;

    /// Get a friendly description of this game.
    fn name(&self) -> &'static str;

    /// Get a reference to the [ParamRepository] of this game.
    fn param_repository(&self) -> &'static ParamRepository;

    /// Enable local file overrides. Colloquially known as the "UXM" patch.
    fn enable_file_overrides(&self) -> bool;
}

/// Given a reference to some [SprjGame], attempt
/// to downcast it to a target game.
pub fn get_game_api<T>(this: &dyn SprjGame) -> Option<&T>
where
    T: SprjGame + 'static,
{
    this.as_any().downcast_ref()
}
