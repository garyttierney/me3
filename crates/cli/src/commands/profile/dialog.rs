use std::{fs, path::PathBuf, rc::Rc};

use me3_mod_protocol::{
    native::Native,
    package::{Package, WithPackageSource},
    Game, ModProfile, Supports,
};
use slint::{ComponentHandle, Model, ModelRc, SharedString};
use tracing::info;

use crate::ui::{ModItem, ProfileDialog};

/// Create a pop-up dialog to edit an `existing` [ModProfile].
///
/// Returns a new [ModProfile] on completion or `None` if the user cancelled making changes.
pub fn show_profile_dialog(existing: ModProfile) -> color_eyre::Result<Option<ModProfile>> {
    let existing_items: Vec<ModItem> = existing
        .natives()
        .iter()
        .map(|item| (true, item.source()))
        .chain(existing.packages().iter().map(|pkg| (false, pkg.source())))
        .map(|(is_dll, path)| ModItem {
            is_dll,
            path: SharedString::from(&*path.to_string_lossy()),
        })
        .collect();

    let model = Rc::new(slint::VecModel::from(existing_items));
    let dialog = ProfileDialog::new()?;
    let games = [Game::EldenRing, Game::Nightreign];
    let game_names = games
        .iter()
        .map(|game| SharedString::from(format!("{game:?}")));

    let selected_game = existing
        .supports()
        .first()
        .and_then(|supports| games.iter().position(|g| g == &supports.game))
        .unwrap_or_default();

    dialog.set_game(selected_game as i32);
    dialog.set_supported_games(ModelRc::new(slint::VecModel::from_iter(game_names)));
    dialog.set_mods(model.clone().into());

    dialog.on_remove_mod({
        let model = model.clone();
        move |offset| {
            model.remove(offset as usize);
        }
    });

    dialog.on_add_new_mod({
        let model = model.clone();
        move || {
            let Some(mod_folder) = rfd::FileDialog::new()
                .set_title("Select a folder containing modded files or DLLs")
                .pick_folder()
            else {
                return;
            };

            let natives: Vec<PathBuf> = fs::read_dir(&mod_folder)
                .unwrap()
                .filter_map(|path| {
                    let entry = path.ok()?;
                    let path = entry.path();
                    let is_dll = path.extension()? == "dll";

                    is_dll.then_some(path)
                })
                .collect();

            for native in natives {
                model.push(ModItem {
                    path: native.to_str().unwrap().into(),
                    is_dll: true,
                });
            }

            model.push(ModItem {
                path: mod_folder.to_str().unwrap().into(),
                is_dll: false,
            });
        }
    });

    fn close(dialog: &ProfileDialog) -> impl Fn() {
        let dialog = dialog.as_weak();
        move || {
            dialog.upgrade().unwrap().hide();
        }
    };

    dialog.on_save(close(&dialog));
    dialog.on_cancel(close(&dialog));
    dialog.show()?;

    slint::run_event_loop()?;
    info!(
        discard_changes = dialog.get_discard_changes(),
        "finished GUI configuration"
    );

    if dialog.get_discard_changes() {
        return Ok(None);
    }

    let mut new_profile = ModProfile::default();
    for item in model.iter() {
        if item.is_dll {
            let natives = new_profile.natives_mut();
            natives.push(Native::new(item.path.as_str()));
        } else {
            let packages = new_profile.packages_mut();
            packages.push(Package::new(item.path.as_str()));
        }
    }

    if dialog.get_game() >= 0 {
        let supports = new_profile.supports_mut();

        supports.push(Supports {
            game: games[dialog.get_game() as usize],
            since_version: None,
        });
    }

    Ok(Some(new_profile))
}
