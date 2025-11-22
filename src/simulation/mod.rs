use bevy::prelude::*;

pub mod draw;
pub mod engine;
pub mod graphics;
pub mod render;
pub mod stats_boards;
pub mod universe;
pub mod view;

use crate::simulation::draw::MouseDrawPlugin;
use crate::simulation::stats_boards::StatsBoardPlugin;

use self::graphics::GraphicsPlugin;
use self::render::SimulationRenderPlugin;
use self::universe::UniversePlugin;
use self::view::ViewPlugin;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ViewPlugin);
        app.add_plugins(GraphicsPlugin);
        app.add_plugins(UniversePlugin);
        app.add_plugins(SimulationRenderPlugin);
        app.add_plugins(MouseDrawPlugin);
        app.add_plugins(StatsBoardPlugin);
    }
}
