use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::simulation::graphics::{GridLayerMaterial, LayerViewport, PixelLayer, PixelLayerBundle};
use crate::simulation::stats_boards::StatsBoard;
use crate::simulation::universe::Universe;
use crate::simulation::view::SimulationView;

pub struct SimulationRenderPlugin;

impl Plugin for SimulationRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_universe_layer)
            .add_systems(Update, render_universe);
    }
}

#[derive(Component)]
struct UniverseLayer;

fn setup_universe_layer(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GridLayerMaterial>>,
) {
    commands.spawn((
        PixelLayerBundle::new(
            &mut images,
            &mut meshes,
            &mut materials,
            0.0,
            Vec4::new(1.0, 1.0, 1.0, 1.0),
            Vec4::new(0.1, 0.1, 0.1, 1.0),
        ),
        UniverseLayer,
    ));
}

fn render_universe(
    universe: Res<Universe>,
    view: Res<SimulationView>,
    mut images: ResMut<Assets<Image>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_layer: Query<&PixelLayer, With<UniverseLayer>>,
    mut stats: ResMut<StatsBoard>,
) {
    let Ok(layer) = q_layer.single() else { return };
    let Some(image) = images.get_mut(&layer.image_handle) else {
        return;
    };
    let Ok(window) = q_window.single() else {
        return;
    };

    let Some(viewport) = LayerViewport::new(window, &view) else {
        return;
    };
    let buffer = viewport.get_buffer(image);

    // Draw
    universe.engine.draw_to_buffer(
        viewport.get_world_rect(),
        buffer,
        viewport.screen_w,
        viewport.screen_h,
    );

    stats.insert("Population", format!("{}", universe.engine.population()));
}
