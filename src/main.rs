mod plugins {
    pub mod camera_movement;
    pub mod clear_screen;
    pub mod mouse_draw;
    pub mod mouse_position;
    pub mod seeded_rng;
}

use std::collections::HashMap;

use crate::plugins::{
    camera_movement::CameraMovementPlugin, clear_screen::ClearScreenPlugin,
    mouse_draw::MouseDrawPlugin, mouse_position::MousePositionPlugin, seeded_rng::SeededRngPlugin,
};

use bevy::{dev_tools::fps_overlay::FpsOverlayPlugin, prelude::*};

const CELL_WIDTH: f32 = 20.;

fn main() {
    let mut app = App::new();

    // Plugins
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            present_mode: bevy::window::PresentMode::AutoNoVsync,
            ..default()
        }),
        ..default()
    }));
    app.add_plugins(FpsOverlayPlugin::default());
    app.add_plugins(SeededRngPlugin::new([42; 32]));
    app.add_plugins(ClearScreenPlugin);
    app.add_plugins(MousePositionPlugin);
    app.add_plugins(MouseDrawPlugin);
    app.add_plugins(CameraMovementPlugin);

    // Tick timer
    app.insert_resource(TickTimer(Timer::from_seconds(0.03, TimerMode::Repeating)));

    // Systems
    app.add_systems(Startup, setup_assets);
    app.add_systems(Startup, (spawn_camera, spawn_pentomino).after(setup_assets));
    app.add_systems(FixedUpdate, tick);

    // Observers
    app.add_observer(spawn_cell_observer);

    app.run();
}

#[derive(Resource)]
struct TickTimer(Timer);

#[derive(Resource)]
struct CellAssets {
    mesh: Handle<Mesh>,
    alive_material: Handle<ColorMaterial>,
    ghost_material: Handle<ColorMaterial>,
}

#[derive(Component)]
pub struct Cell;

#[derive(Component, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Position {
    x: i32,
    y: i32,
}

fn setup_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.insert_resource(CellAssets {
        mesh: meshes.add(Rectangle::new(CELL_WIDTH, CELL_WIDTH)),
        alive_material: materials.add(Color::oklch(0.9, 0.0, 0.0)),
        ghost_material: materials.add(Color::oklch(0.5, 0.0, 0.0)),
    });
}

#[derive(Event)]
struct SpawnCellEvent {
    position: Position,
}

impl SpawnCellEvent {
    fn new(x: i32, y: i32) -> Self {
        Self {
            position: Position { x, y },
        }
    }
}

fn spawn_cell_observer(
    event: On<SpawnCellEvent>,
    mut commands: Commands,
    cell_assets: Res<CellAssets>,
) {
    commands.spawn((
        Cell,
        event.position,
        Mesh2d(cell_assets.mesh.clone()),
        MeshMaterial2d(cell_assets.alive_material.clone()),
        Transform::from_xyz(
            event.position.x as f32 * CELL_WIDTH,
            event.position.y as f32 * CELL_WIDTH,
            1.0,
        ),
    ));
}

fn spawn_blinker(mut commands: Commands) {
    commands.trigger(SpawnCellEvent::new(-1, 0));
    commands.trigger(SpawnCellEvent::new(0, 0));
    commands.trigger(SpawnCellEvent::new(1, 0));
}

fn spawn_pentomino(mut commands: Commands) {
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
        commands.trigger(SpawnCellEvent::new(x, y));
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}

fn neighbors(pos: &Position) -> impl Iterator<Item = Position> + '_ {
    (-1..=1)
        .flat_map(move |dx| (-1..=1).map(move |dy| (dx, dy)))
        .filter(|&(dx, dy)| dx != 0 || dy != 0)
        .map(move |(dx, dy)| Position {
            x: pos.x + dx,
            y: pos.y + dy,
        })
}

fn tick(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<TickTimer>,
    cells: Query<(Entity, &Position), With<Cell>>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        let counts: HashMap<Position, (Option<Entity>, u8)> =
            cells.iter().fold(HashMap::new(), |mut acc, (entity, pos)| {
                for neighbor in neighbors(pos) {
                    let entry = acc.entry(neighbor).or_insert((None, 0));
                    entry.1 += 1;
                }

                let self_entry = acc.entry(*pos).or_insert((None, 0));
                self_entry.0 = Some(entity);

                acc
            });

        counts
            .iter()
            .for_each(|(pos, (entity_opt, count))| match entity_opt {
                Some(entity) if *count != 2 && *count != 3 => {
                    commands.entity(*entity).despawn();
                }
                None if *count == 3 => {
                    commands.trigger(SpawnCellEvent::new(pos.x, pos.y));
                }
                _ => {}
            });
    };
}
