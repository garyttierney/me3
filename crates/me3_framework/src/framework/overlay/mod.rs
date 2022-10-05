use std::sync::{Mutex, RwLock};

use egui::{Align2, Color32, Key, Pos2, Ui};
use once_cell::sync::OnceCell;

use super::{FrameworkError, FrameworkGlobal};

mod hook;

pub struct OverlayPanel {
    renderer: Box<dyn Fn(&mut Ui) + Send + Sync>,
    title: String,
}

struct OverlayState {
    hidden: bool,
    about: bool,
}

pub struct Overlay {
    panels: RwLock<Vec<OverlayPanel>>,
    state: Mutex<OverlayState>,
}

impl FrameworkGlobal for Overlay {
    fn cell() -> &'static OnceCell<Self> {
        static INSTANCE: OnceCell<Overlay> = OnceCell::new();
        &INSTANCE
    }

    fn create() -> Result<Self, FrameworkError> {
        hook::install_overlay_hooks()?;

        Ok(Overlay {
            panels: RwLock::new(vec![]),
            state: Mutex::new(OverlayState {
                hidden: true,
                about: true,
            }),
        })
    }
}

impl Overlay {
    pub fn render(&self, context: &egui::Context) {
        let mut state = self.state.lock().unwrap();

        if context.input().key_pressed(Key::F1) {
            state.hidden = !state.hidden;
        }

        if context.input().key_pressed(Key::F11) {
            state.about = !state.about;
        }

        if state.about {
            context.debug_painter().debug_text(
                Pos2::new(10.0, 10.0),
                Align2::LEFT_TOP,
                Color32::WHITE,
                format!("me3 v{}", env!("CARGO_PKG_VERSION")),
            );
        }

        if !state.hidden {
            let panels = self
                .panels
                .read()
                .expect("panel lock was poisoned by writer");

            for panel in &*panels {
                egui::containers::Window::new(&panel.title)
                    .show(context, |ui| (panel.renderer)(ui));
            }
        }
    }

    pub fn register_panel<F>(&self, title: String, renderer: F)
    where
        F: Fn(&mut Ui) + Send + Sync + 'static,
    {
        self.panels
            .write()
            .expect("panel lock was poisoned by previous writer")
            .push(OverlayPanel {
                title,
                renderer: Box::new(renderer),
            });
    }
}
