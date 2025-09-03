use crate::app::Me3App;

pub mod natives;
pub mod properties;
pub mod save_file;
pub mod skip_logos;
pub mod vfs;

pub trait Plugin {
    fn build(&self, app: &mut Me3App);
}
