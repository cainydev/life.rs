use crate::simulation::rendering::BitChunkMaterial;
#[allow(unused_imports)]
use crate::simulation::systems::_draw_chunks_debug;

use super::{
    rendering::ChunkRenderCache, rendering::render_chunks, systems::tick_universe,
    universe::Universe,
};
use bevy::{asset::TrackAssets, prelude::*, sprite_render::Material2dPlugin};

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Universe>();
        app.init_resource::<ChunkRenderCache>();

        app.add_plugins(Material2dPlugin::<BitChunkMaterial>::default());

        app.add_systems(FixedUpdate, tick_universe);

        //app.add_systems(Update, _draw_chunks_debug);
        app.add_systems(Update, render_chunks);
    }
}
