use std::sync::{Mutex, RwLock};

pub use egui::*;
use once_cell::sync::OnceCell;

use super::{FrameworkError, FrameworkGlobal};

mod hook;

// not really, but OverlayComponent is only accessed from a single thread
// needs a better design
unsafe impl Sync for OverlayComponent {}
pub struct OverlayComponent {
    renderer: Box<dyn FnMut(&Context) + Send>,
}

struct OverlayState {
    hidden: bool,
    about: bool,
}

pub struct Overlay {
    panels: RwLock<Vec<OverlayComponent>>,
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

        if context.input().key_pressed(Key::F2) {
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
            // TODO: locking here is too coarse, it should lock individual panels
            let mut panels = self
                .panels
                .write()
                .expect("panel lock was poisoned by writer");

            for panel in &mut *panels {
                (panel.renderer)(context);
            }
        }
    }

    pub fn register_component<F>(&self, renderer: F)
    where
        F: FnMut(&Context) + Send + 'static,
    {
        self.panels
            .write()
            .expect("panel lock was poisoned by previous writer")
            .push(OverlayComponent {
                renderer: Box::new(renderer),
            });
    }

    pub fn register_panel<F>(&self, title: &'static str, mut renderer: F)
    where
        F: FnMut(&mut Ui) + Send + 'static,
    {
        self.register_component(move |ctx| {
            egui::containers::Window::new(title)
                .resizable(true)
                .collapsible(false)
                .default_size([250.0, 150.0])
                .title_bar(false)
                .show(ctx, |ui| (renderer)(ui));
        });
    }
}
