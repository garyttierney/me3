use me3_framework::scripting::ScriptHost;
use me3_game_support_fromsoft::sprj::SprjGame;

use crate::widgets::Console;

pub trait GameSupport<T: SprjGame> {
    /// Attempt to initialize the [SprjGame] instance from the current process.
    ///1 If the current process does not match the game this bootstrap represents, then
    /// [None] is returned.
    fn initialize() -> Option<&'static T>;

    #[allow(unused)]
    fn configure_console(game: &'static T, console: &mut Console) {}

    #[allow(unused)]
    fn configure_scripting(game: &'static T, scripting: &ScriptHost) {}
}
