use std::sync::LazyLock;

use me3_env::GameVars;
use me3_mod_protocol::Game;

pub static GAME: LazyLock<Game> = LazyLock::new(|| {
    let GameVars { launched } = me3_env::deserialize_from_env().expect("ME3_GAME not set");
    launched
});
