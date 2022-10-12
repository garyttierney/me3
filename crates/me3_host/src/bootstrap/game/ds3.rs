use me3_game_support_ds3::DarkSouls3;
use me3_game_support_fromsoft::impl_param_file_descriptor;
use serde::{Deserialize, Serialize};

use crate::{bootstrap::game_support::GameSupport, script_api::LuaParamAccessor};

pub struct Ds3Bootstrap;

impl GameSupport<DarkSouls3> for Ds3Bootstrap {
    fn initialize() -> Option<&'static DarkSouls3> {
        Some(&DarkSouls3)
    }

    fn configure_scripting(
        game: &'static DarkSouls3,
        scripting: &me3_framework::scripting::ScriptHost,
    ) {
        if let Err(e) = scripting.set_table("params", |table| {
            table.set(
                "network_area",
                LuaParamAccessor::<NetworkAreaParam>::new(game),
            )?;

            Ok(table)
        }) {
            log::error!("{:?}", e);
        }
    }
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct NetworkAreaParam {
    cell_size_x: f32,
    cell_size_y: f32,
    cell_size_z: f32,
    cell_offset_x: f32,
    cell_offset_y: f32,
    cell_offset_z: f32,
}

impl_param_file_descriptor!(NetworkAreaParam, 48);
