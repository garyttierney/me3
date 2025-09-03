use std::{
    collections::HashMap,
    mem, slice,
    sync::{Arc, Mutex},
    vec::Vec as StdVec,
};

use bevy_app::PostStartup;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{resource::Resource, system::Res};
use eyre::OptionExt;
use me3_launcher_attach_protocol::AttachConfig;
use me3_mod_host_types::string::DlUtf16String;
use me3_mod_protocol::Game;
use pelite::pe::Pe;
use rdvec::Vec;
use regex::bytes::Regex;
use windows::core::PCWSTR;

use crate::{app::ExternalRes, executable::Executable, hook, plugins::Plugin};

pub struct GamePropertiesPlugin;

#[derive(Default, Deref, DerefMut, Resource)]
pub struct GameProperties(Arc<Mutex<HashMap<StdVec<u16>, bool>>>);

type GetBoolProperty = unsafe extern "C" fn(usize, *const (), bool) -> bool;

fn bool_property_getter(
    attach_config: &AttachConfig,
    exe: &Executable,
) -> Result<GetBoolProperty, eyre::Error> {
    // Matches callsites for the boolean DLSystemProperty getter.
    //
    // In Dark Souls 3, Sekiro and ER, the getter takes in a reference to a DLString,
    // in later games, it was changed to a nul terminated UTF-16 string pointer.
    //
    // The patterns match loading the pointer to the `std::map` containing the property names
    // and their values loaded in RCX, the queried property name in RDX and true/false in R8B
    // as the default value in the case of the property missing from the map.
    let function_call_re_str = match attach_config.game {
        Game::DarkSouls3 => {
            r"(?s-u)(?:\x48\x8d\x54\x24\x30\x48\x8b\x0d.{4}\xe8(.{4})\x88\x05.{4}\x48\x83\x7c\x24\x48\x08\x72.)|(?:\x48\x8d\x54\x24\x30\x48\x8b\x0d.{4}\xe8(.{4})\x0f\xb6\xd8\x48\x83\x7c\x24\x48\x08\x72.)"
        }
        Game::Sekiro | Game::EldenRing => {
            r"(?s-u)(?:\x48\x8d\x54\x24\x30\x48\x8b\x0d.{4}\xe8(.{4})\x88\x05.{4}\x48\x83\x7c\x24\x50\x08\x72.)|(?:\x48\x8d\x54\x24\x30\x48\x8b\x0d.{4}\xe8(.{4})\x0f\xb6\xd8\x48\x83\x7c\x24\x50\x08\x72.)"
        }
        Game::ArmoredCore6 | Game::Nightreign => {
            r"(?s-u)(?:(?:\x45\x33\xc0)|(?:\x41\xb0\x01))\x48\x8d\x15.{4}\x48\x8b\x0d.{4}\xe8(.{4})"
        }
    };

    let function_call_re = Regex::new(function_call_re_str).unwrap();

    let text = exe.get_section_bytes(
        exe.section_headers()
            .by_name(".text")
            .ok_or_eyre(".text section is missing")?,
    )?;

    // The above patterns return a lot of matches, filter by the most common one
    // to definitively pick the right match.
    function_call_re
        .captures_iter(text)
        .map(|c| {
            let [call_disp32] = c.extract().1;
            let call_bytes = <[u8; 4]>::try_from(call_disp32).unwrap();

            call_disp32
                .as_ptr_range()
                .end
                .wrapping_byte_offset(i32::from_le_bytes(call_bytes) as _)
        })
        .fold(HashMap::<_, usize>::new(), |mut map, ptr| {
            *map.entry(ptr).or_default() += 1;
            map
        })
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(ptr, _)| unsafe { mem::transmute::<_, GetBoolProperty>(ptr) })
        .ok_or_eyre("pattern returned no matches")
}

impl GamePropertiesPlugin {
    pub fn hook_property_getter(
        attach_config: ExternalRes<AttachConfig>,
        exe: ExternalRes<Executable>,
        properties: Res<GameProperties>,
    ) -> bevy_ecs::error::Result {
        let get_bool_property = bool_property_getter(&attach_config, &exe)?;
        let game = attach_config.game;
        let props = properties.clone();

        hook!(
            pointer = get_bool_property,
            move |p1, name, default, trampoline| unsafe {
                if name.is_null() {
                    return false;
                }

                let property = if game >= Game::ArmoredCore6 {
                    let name = PCWSTR::from_raw(name as *const u16);
                    slice::from_raw_parts(name.as_ptr(), name.len())
                } else {
                    let name = &*(name as *const DlUtf16String);
                    name.get().unwrap().as_slice()
                };

                props
                    .lock()
                    .unwrap()
                    .get(property)
                    .copied()
                    .unwrap_or_else(|| trampoline(p1, name, default))
            }
        )?;

        Ok(())
    }
}

impl Plugin for GamePropertiesPlugin {
    fn build(&self, app: &mut crate::app::Me3App) {
        app.init_resource::<GameProperties>();
        app.register_system(PostStartup, Self::hook_property_getter);
    }
}
