mod simulation;

use bevy::math::I64Vec2;
use bevy::prelude::*;

use crate::simulation::SimulationPlugin;
use crate::simulation::universe::Universe;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            canvas: Some("#bevy-canvas".into()),
            ..default()
        }),
        ..default()
    }));

    //app.add_plugins(FpsOverlayPlugin::default());
    app.insert_resource(Time::<Fixed>::from_hz(30.0));

    app.add_plugins(SimulationPlugin);

    app.add_systems(Startup, spawn_camera);
    app.add_systems(Startup, spawn_initial_pattern);

    app.run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Transform::default()));
}

fn spawn_initial_pattern(mut universe: ResMut<Universe>) {
    let coords = vec![
        I64Vec2 { x: -4, y: 0 },
        I64Vec2 { x: -4, y: -1 },
        I64Vec2 { x: -3, y: -2 },
        I64Vec2 { x: -2, y: -3 },
        I64Vec2 { x: -1, y: -4 },
        I64Vec2 { x: 0, y: -4 },
        I64Vec2 { x: 1, y: -3 },
        I64Vec2 { x: 2, y: -2 },
        I64Vec2 { x: 3, y: -1 },
        I64Vec2 { x: 3, y: 0 },
        I64Vec2 { x: 2, y: 1 },
        I64Vec2 { x: 1, y: 2 },
        I64Vec2 { x: 0, y: 3 },
        I64Vec2 { x: -1, y: 3 },
        I64Vec2 { x: -2, y: 2 },
        I64Vec2 { x: -3, y: 1 },
    ];

    universe.add_cells(coords);
}
