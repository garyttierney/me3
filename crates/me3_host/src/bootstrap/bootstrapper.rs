use me3_framework::Framework;
use me3_game_support_fromsoft::sprj::SprjGame;

use super::{game::Ds3Bootstrap, game_support::GameSupport};
use crate::widgets::Console;

type GameBootstrapRunner = dyn FnOnce(&Framework, &mut Console) -> Option<&'static dyn SprjGame>;

pub struct Bootstrapper {
    game_bootstraps: Vec<Box<GameBootstrapRunner>>,
}

impl Bootstrapper {
    pub fn new() -> Self {
        Bootstrapper {
            game_bootstraps: vec![],
        }
    }

    pub fn with_game_support<G, Support>(mut self) -> Self
    where
        G: SprjGame + 'static,
        Support: GameSupport<G> + ?Sized,
    {
        self.game_bootstraps.push(Box::new(|framework, console| {
            Support::initialize().map(|api| {
                Support::configure_console(api, console);
                Support::configure_scripting(api, framework.get_script_host());

                api as &dyn SprjGame
            })
        }));
        self
    }

    pub fn bootstrap(
        self,
        framework: &Framework,
        console: &mut Console,
    ) -> Option<&'static dyn SprjGame> {
        self.game_bootstraps
            .into_iter()
            .find_map(|bootstrap| bootstrap(framework, console))
    }
}

/// Infer the current game that is running from the environment and bootstrap an instance of [SprjGame] for the game.
pub fn bootstrap_game(
    framework: &Framework,
    console: &mut Console,
) -> Option<&'static dyn SprjGame> {
    Bootstrapper::new()
        .with_game_support::<_, Ds3Bootstrap>()
        .bootstrap(framework, console)
}
