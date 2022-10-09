//! Bootstrap procedures for specific games.

use me3_framework::Framework;
use me3_game_support_ds3::DarkSouls3;
use me3_game_support_fromsoft::sprj::SprjGame;

/// Infer the current game that is running from the environment and bootstrap an instance of [SprjGame] for the game.
pub fn bootstrap_game(_framework: &Framework) -> Option<&'static dyn SprjGame> {
    // TODO: infer game from process, currently assumes DS3.
    Some(&DarkSouls3)
}
