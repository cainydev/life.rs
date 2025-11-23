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
    // let draw_start = Time<Real>

    universe.draw_to_buffer(
        viewport.get_world_rect(),
        buffer,
        viewport.screen_w,
        viewport.screen_h,
    );

    // let draw_duration = draw_start.elapsed();

    stats.insert("Population", format_metric(universe.population()));
    // stats.insert(
    //     "Draw Time",
    //     format!("{:.2} ms", draw_duration.as_micros() as f64 / 1000.0),
    // );
}

fn format_metric(count: u64) -> String {
    if count < 1_000 {
        return count.to_string();
    }

    let suffixes = ["k", "M", "B", "T", "Q"]; // Thousand, Million, Billion, Trillion, Quadrillion
    let mut value = count as f64;
    let mut suffix_idx = 0;

    // Divide by 1000 until the number is small enough
    while value >= 1_000.0 && suffix_idx < suffixes.len() {
        value /= 1_000.0;
        suffix_idx += 1;
    }

    // Format to 2 decimal places
    let formatted = format!("{:.2}", value);

    // Clean up trailing zeros and decimal point (e.g., "150.00" -> "150", "2.50" -> "2.5")
    let cleaned = formatted.trim_end_matches('0').trim_end_matches('.');

    format!("{}{}", cleaned, suffixes[suffix_idx - 1])
}
