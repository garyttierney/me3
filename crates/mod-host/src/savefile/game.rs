use std::path::PathBuf;

use me3_mod_protocol::Game;

pub fn savefile_dir(game: Game) -> Option<PathBuf> {
    use Game::*;

    // DS1/DSR will need to resolve CSIDL_MYDOCUMENTS/FOLDERID_Documents instead.
    let base_dir = match game {
        DarkSouls3 | Sekiro | EldenRing | ArmoredCore6 | Nightreign => {
            std::env::var_os("APPDATA").map(PathBuf::from)
        }
    };

    Some(match game {
        DarkSouls3 => base_dir?.join("DarkSoulsIII"),
        Sekiro => base_dir?.join("Sekiro"),
        EldenRing => base_dir?.join("EldenRing"),
        ArmoredCore6 => base_dir?.join("ArmoredCore6"),
        Nightreign => base_dir?.join("Nightreign"),
    })
}
