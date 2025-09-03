use std::sync::{Arc, RwLock};

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    resource::Resource,
    schedule::{ExecutorKind, IntoScheduleConfigs, Schedule, ScheduleLabel},
    system::{Res, ResMut, ScheduleSystem},
    world::World,
};

use crate::plugins::Plugin;

#[derive(Deref, DerefMut)]
pub struct Me3App {
    world: World,
}

/// The `PreStartup` schedule is the first schedule to be executed during initialization. Prefer
/// `[Startup]` unless you specifically need your code to run before any other me3 patches.
#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub struct PreStartup;

/// The `Startup` schedule executes before execution is passed to the game, but after
/// `[PreStartup]`.
#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub struct Startup;

/// The `PostStartup` schedule executes after the game has executed its own `WinMain` startup
/// routine and initialized Steam (if applicable).
#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub struct PostStartup;

/// The `Update` schedule is ran on every frame of the game.
#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub struct Update;

impl Default for Me3App {
    fn default() -> Self {
        Self::new()
    }
}

impl Me3App {
    pub fn new() -> Self {
        let mut world = World::new();
        let mut post_startup_schedule = Schedule::new(PostStartup);
        post_startup_schedule.set_executor_kind(ExecutorKind::SingleThreaded);

        let mut startup_schedule = Schedule::new(Startup);
        startup_schedule.set_executor_kind(ExecutorKind::MultiThreaded);

        let mut pre_startup_schedule = Schedule::new(PreStartup);
        pre_startup_schedule.set_executor_kind(ExecutorKind::SingleThreaded);

        world.add_schedule(pre_startup_schedule);
        world.add_schedule(startup_schedule);
        world.add_schedule(post_startup_schedule);

        Self { world }
    }

    pub fn register_plugin<P>(&mut self, plugin: P)
    where
        P: Plugin,
    {
        plugin.build(self)
    }

    pub fn register_system<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) {
        self.world.schedule_scope(schedule, |_, sched| {
            sched.add_systems(systems);
        });
    }

    pub fn run_schedule(&mut self, label: impl ScheduleLabel) {
        self.world.run_schedule(label);
    }
}

#[derive(Deref, DerefMut, Resource)]
pub struct ExternalResource<T>(pub T);
pub type ExternalRes<'w, T> = Res<'w, ExternalResource<T>>;
pub type ExternalResMut<'w, T> = ResMut<'w, ExternalResource<T>>;

#[derive(Deref, DerefMut, Resource)]
pub struct SharedResource<T>(pub Arc<RwLock<T>>);
pub type SharedRes<'w, T> = Res<'w, SharedResource<T>>;
