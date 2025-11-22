mod simulation;

use bevy::{dev_tools::fps_overlay::FpsOverlayPlugin, prelude::*};

use crate::simulation::SimulationPlugin;
use crate::simulation::universe::Universe;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            present_mode: bevy::window::PresentMode::AutoNoVsync,
            ..default()
        }),
        ..default()
    }));

    app.add_plugins(FpsOverlayPlugin::default());
    app.insert_resource(Time::<Fixed>::from_hz(3.0));

    app.add_plugins(SimulationPlugin);

    app.add_systems(Startup, spawn_camera);
    app.add_systems(Startup, spawn_initial_pattern);

    app.run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Transform::default()));
}

fn spawn_initial_pattern(mut universe: ResMut<Universe>) {
    let coords = [
        (-4, 0),
        (-4, -1),
        (-3, -2),
        (-2, -3),
        (-1, -4),
        (0, -4),
        (1, -3),
        (2, -2),
        (3, -1),
        (3, 0),
        (2, 1),
        (1, 2),
        (0, 3),
        (-1, 3),
        (-2, 2),
        (-3, 1),
    ];

    for (x, y) in coords {
        universe.set_cell(IVec2::new(x, y), true);
    }
}
