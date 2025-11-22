use bevy::prelude::*;
use bevy::{
    ecs::{
        resource::Resource,
        system::{Res, ResMut},
    },
    input::{ButtonInput, keyboard::KeyCode},
    math::IVec2,
};

use super::{engine::LifeEngine, engine::hashlife::hashlife::Hashlife};

pub struct UniversePlugin;

impl Plugin for UniversePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Universe>()
            .add_systems(FixedUpdate, step_universe)
            .add_systems(PreUpdate, clear_universe);
    }
}

#[derive(Resource)]
pub struct Universe {
    pub engine: Box<dyn LifeEngine + Send + Sync>,
}

impl Default for Universe {
    fn default() -> Self {
        Self {
            engine: Box::new(Hashlife::new()),
        }
    }
}

impl Universe {
    pub fn set_cell(&mut self, global_pos: IVec2, value: bool) {
        self.engine
            .set_cell(global_pos.x as i64, global_pos.y as i64, value);
    }
}

fn step_universe(mut universe: ResMut<Universe>) {
    universe.engine.step(1);
}

fn clear_universe(mut universe: ResMut<Universe>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::KeyC) {
        universe.engine.clear();
        println!("Universe cleared!");
    }
}
