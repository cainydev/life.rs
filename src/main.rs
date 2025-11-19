mod simulation;
mod plugins {
    pub mod camera_movement;
    pub mod clear_screen;
    pub mod mouse_draw;
    pub mod mouse_position;
}

use bevy::{dev_tools::fps_overlay::FpsOverlayPlugin, prelude::*};

use crate::plugins::{
    camera_movement::CameraMovementPlugin, clear_screen::ClearScreenPlugin,
    mouse_draw::MouseDrawPlugin, mouse_position::MousePositionPlugin,
};

use crate::simulation::plugin::SimulationPlugin;
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
    app.insert_resource(Time::<Fixed>::from_hz(144.0));

    // 2. Game Plugin
    app.add_plugins(SimulationPlugin);

    // 3. Tools & Interaction
    app.add_plugins(ClearScreenPlugin);
    app.add_plugins(MousePositionPlugin);
    app.add_plugins(MouseDrawPlugin);
    app.add_plugins(CameraMovementPlugin);

    // 4. Setup Systems
    app.add_systems(Startup, spawn_camera);
    app.add_systems(Startup, spawn_initial_pattern);

    app.run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(0.5)),
    ));
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
