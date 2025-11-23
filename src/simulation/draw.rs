use bevy::math::I64Vec2;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::simulation::graphics::{GridLayerMaterial, LayerViewport, PixelLayer, PixelLayerBundle};
use crate::simulation::universe::Universe;
use crate::simulation::view::{MouseWorldPosition, SimulationView};

pub struct MouseDrawPlugin;

impl Plugin for MouseDrawPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DrawingBuffer>()
            .add_systems(Startup, setup_draw_layer)
            .add_systems(Update, (accumulate_drawing, commit_drawing, render_overlay));
    }
}

#[derive(Resource, Default)]
struct DrawingBuffer {
    pub positions: HashSet<I64Vec2>,
    pub last_pos: Option<I64Vec2>,
}

#[derive(Component)]
struct DrawLayer;

fn setup_draw_layer(
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
            0.1, // Z-Index 0.1
            Vec4::new(0.0, 1.0, 1.0, 0.6),
            Vec4::new(0.0, 0.0, 0.0, 0.0),
        ),
        DrawLayer,
    ));
}

fn accumulate_drawing(
    mut buffer: ResMut<DrawingBuffer>,
    mouse_res: Res<MouseWorldPosition>,
    buttons: Res<ButtonInput<MouseButton>>,
) {
    if !buttons.pressed(MouseButton::Left) {
        buffer.last_pos = None;
        return;
    }

    let Some(cur_pos) = mouse_res.grid_pos else {
        return;
    };

    let prev_pos = buffer.last_pos.unwrap_or(cur_pos);

    let mut x = prev_pos.x;
    let mut y = prev_pos.y;
    let dx = (cur_pos.x - prev_pos.x).abs();
    let dy = (cur_pos.y - prev_pos.y).abs();
    let sx = if prev_pos.x < cur_pos.x { 1 } else { -1 };
    let sy = if prev_pos.y < cur_pos.y { 1 } else { -1 };
    let mut err = (if dx > dy { dx } else { -dy }) / 2;

    loop {
        buffer.positions.insert(I64Vec2::new(x, y));
        if x == cur_pos.x && y == cur_pos.y {
            break;
        }
        let e2 = err;
        if e2 > -dx {
            err -= dy;
            x += sx;
        }
        if e2 < dy {
            err += dx;
            y += sy;
        }
    }
    buffer.last_pos = Some(cur_pos);
}

fn commit_drawing(
    mut universe: ResMut<Universe>,
    mut buffer: ResMut<DrawingBuffer>,
    buttons: Res<ButtonInput<MouseButton>>,
) {
    if !buttons.pressed(MouseButton::Left) && !buffer.positions.is_empty() {
        let points: Vec<I64Vec2> = buffer.positions.drain().collect();
        universe.add_cells(points);
    }
}

fn render_overlay(
    mut images: ResMut<Assets<Image>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_layer: Query<&PixelLayer, With<DrawLayer>>,
    view: Res<SimulationView>,
    buffer: Res<DrawingBuffer>,
    mouse_res: Res<MouseWorldPosition>,
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
    let pixel_buffer = viewport.get_buffer(image);

    // Clear and Draw
    pixel_buffer.fill(0);

    for &pos in &buffer.positions {
        viewport.draw_cell(pixel_buffer, pos.x as i64, pos.y as i64, 255);
    }
    if let Some(pos) = mouse_res.grid_pos {
        viewport.draw_cell(pixel_buffer, pos.x as i64, pos.y as i64, 255);
    }
}
