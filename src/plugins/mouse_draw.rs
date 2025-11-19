use crate::plugins::mouse_position::{MouseGridPosition, update_mouse_position};
use crate::simulation::coords::{CELL_WIDTH, cell_to_world};
use crate::simulation::universe::Universe;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

pub struct MouseDrawPlugin;

impl Plugin for MouseDrawPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DrawingBuffer>(); // Unser temporärer Speicher
        app.add_systems(
            Update,
            (
                draw_hover_cursor,   // Zeigt aktuellen Maus-Cursor
                accumulate_drawing,  // Sammelt Punkte beim Ziehen in den Buffer
                draw_buffer_preview, // Malt den Buffer (Vorschau)
                commit_drawing,      // Überträgt Buffer ins Universum beim Loslassen
            )
                .after(update_mouse_position),
        );
    }
}

#[derive(Resource, Default)]
struct DrawingBuffer {
    pub positions: HashSet<IVec2>,
}

fn draw_hover_cursor(mouse_position_res: Res<MouseGridPosition>, mut gizmos: Gizmos) {
    if let Some(pos) = mouse_position_res.cur {
        gizmos.rect_2d(
            Isometry2d::from_translation(cell_to_world(pos)),
            Vec2::splat(CELL_WIDTH * 0.9),
            Color::srgb(0.5, 0.5, 0.5).with_alpha(0.3),
        );
    }
}

fn accumulate_drawing(
    mut buffer: ResMut<DrawingBuffer>,
    mouse_position_res: Res<MouseGridPosition>,
    buttons: Res<ButtonInput<MouseButton>>,
) {
    if !buttons.pressed(MouseButton::Left) {
        return;
    }

    let cur_pos = match mouse_position_res.cur {
        Some(p) => p,
        None => return,
    };

    let prev_pos = if buttons.just_pressed(MouseButton::Left) {
        cur_pos
    } else {
        mouse_position_res.prev.unwrap_or(cur_pos)
    };

    // --- Bresenham Algorithmus (Linieninterpolation) ---
    // Verhindert Lücken im Buffer bei schnellen Bewegungen
    let mut x = prev_pos.x;
    let mut y = prev_pos.y;

    let dx = (cur_pos.x - prev_pos.x).abs();
    let dy = (cur_pos.y - prev_pos.y).abs();

    let sx = if prev_pos.x < cur_pos.x { 1 } else { -1 };
    let sy = if prev_pos.y < cur_pos.y { 1 } else { -1 };

    let mut err = if dx > dy { dx } else { -dy } / 2;

    loop {
        // WICHTIG: Wir schreiben in den Buffer, NICHT ins Universe
        buffer.positions.insert(IVec2::new(x, y));

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
}

fn draw_buffer_preview(buffer: Res<DrawingBuffer>, mut gizmos: Gizmos) {
    for &pos in &buffer.positions {
        let center = Vec2::new(
            pos.x as f32 * CELL_WIDTH + (CELL_WIDTH / 2.0),
            pos.y as f32 * CELL_WIDTH + (CELL_WIDTH / 2.0),
        );

        gizmos.rect_2d(
            Isometry2d::from_translation(center),
            Vec2::splat(CELL_WIDTH * 0.9),
            Color::srgb(0.0, 0.8, 0.8).with_alpha(0.8),
        );
    }
}

fn commit_drawing(
    mut universe: ResMut<Universe>,
    mut buffer: ResMut<DrawingBuffer>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    if !mouse.pressed(MouseButton::Left)
        && !keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
    {
        if !buffer.positions.is_empty() {
            for pos in buffer.positions.drain() {
                universe.set_cell(pos, true);
            }
        }
    }
}
