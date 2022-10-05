mod param;

use std::any::Any;

pub use param::{ParamFileDescriptor, ParamRepository};

pub trait SprjGame {
    fn as_any(&self) -> &dyn Any;

    /// Get a reference to the [ParamRepository] of this game.
    fn param_repository(&self) -> &'static ParamRepository;

    /// colloquially known as the "UXM" patch.
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
