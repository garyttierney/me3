use std::{future::IntoFuture, sync::Arc};

use faithe::{internal::alloc_console, pattern::Pattern};
use once_cell::sync::OnceCell;
use thiserror::Error;
use tokio::{runtime::Runtime, task::JoinHandle};

use self::{hooks::Hooks, overlay::Overlay, tracing::Profiler, vfs::VirtualFileSystem};

pub mod hooks;
pub mod overlay;
pub mod tracing;
pub mod vfs;

#[derive(Debug, Error)]
pub enum FrameworkError {
    #[error("failed to patch code")]
    CodePatchingFailed(#[from] detour::Error),

    #[error("internal I/O error")]
    IoError(#[from] std::io::Error),

    #[error("no matches found for '{}' with pattern '{:?}'", identifier, pattern)]
    NoMatchesFound {
        identifier: &'static str,
        pattern: Pattern,
    },

    #[error("unable to resolve symbol '{}'", 0)]
    NoSymbolFound(&'static str),

    #[error("failed to execute pattern scan")]
    PatternScanningFailed(#[from] faithe::FaitheError),
}

pub struct FrameworkBuilder {
    debug_console: bool,
}

impl Default for FrameworkBuilder {
    fn default() -> Self {
        Self {
            debug_console: true,
        }
    }
}

impl FrameworkBuilder {
    pub fn debug_console(self, new_value: bool) -> Self {
        Self {
            debug_console: new_value,
            ..self
        }
    }

    pub fn build(self) -> Result<Me3, FrameworkError> {
        Ok(Arc::new(Framework::setup_framework(self)?))
    }
}

pub type Me3 = std::sync::Arc<Framework>;

pub struct Framework {
    hooks: &'static Hooks,
    overlay: &'static Overlay,
    profiler: &'static Profiler,
    scheduler: Runtime,
    vfs: &'static VirtualFileSystem,
}

pub trait FrameworkGlobal: Sync + Send + Sized {
    fn cell() -> &'static OnceCell<Self>;
    fn create() -> Result<Self, FrameworkError>;

    fn get_or_create() -> Result<&'static Self, FrameworkError> {
        Self::cell().get_or_try_init(|| Self::create())
    }

    // UNSAFE: no guarantee the global was ever created.
    unsafe fn get_unchecked() -> &'static Self {
        Self::cell().get_unchecked()
    }
}

impl Framework {
    pub fn setup_framework(builder: FrameworkBuilder) -> Result<Self, FrameworkError> {
        if builder.debug_console {
            alloc_console().expect("failed to create debug console");
        }

        fern::Dispatch::new()
            .format(move |out, message, record| {
                out.finish(format_args!(
                    "{}[{}] {}",
                    chrono::Local::now().format("[%H:%M:%S]"),
                    record.level(),
                    message
                ))
            })
            .level(log::LevelFilter::Debug)
            .chain(std::io::stdout())
            .chain(fern::log_file("me3.log")?)
            .apply()
            .unwrap_or_else(|_| println!("unable to setup logging system"));

        let hooks = Hooks::get_or_create()?;
        let overlay = Overlay::get_or_create()?;
        let profiler = Profiler::get_or_create()?;
        let scheduler = Runtime::new()?;
        let vfs = VirtualFileSystem::get_or_create()?;

        Ok(Self {
            hooks,
            overlay,
            profiler,
            scheduler,
            vfs,
        })
    }

    pub fn get_overlay(&self) -> &'static overlay::Overlay {
        self.overlay
    }

    pub fn get_profiler(&self) -> &'static tracing::Profiler {
        self.profiler
    }

    pub fn get_vfs(&self) -> &'static vfs::VirtualFileSystem {
        self.vfs
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<<F as IntoFuture>::Output>
    where
        F: IntoFuture,
        F::IntoFuture: Send + Sync + 'static,
        F::Output: Send + Sync + 'static,
    {
        self.scheduler.spawn(future.into_future())
    }

    pub fn run_until_shutdown(&self) {
        self.scheduler.block_on(async move {
            tokio::task::yield_now().await;
        });
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    #[test]
    fn framework_is_thread_safe() {
        assert_send::<Framework>();
        assert_sync::<Framework>();
    }
}
