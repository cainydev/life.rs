use std::collections::HashSet;

use bevy::prelude::*;

use crate::{
    CELL_WIDTH, CellAssets, Position, SpawnCellEvent,
    plugins::mouse_position::{MouseGridPosition, update_mouse_position},
    setup_assets,
};

pub struct MouseDrawPlugin;

impl Plugin for MouseDrawPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_hover_ghost.after(setup_assets));
        app.add_systems(
            Update,
            (update_hover_ghost, update_drawing_set).after(update_mouse_position),
        );
    }
}

#[derive(Component)]
struct HoverGhost;

#[derive(Component)]
struct DrawingGhost;

fn spawn_hover_ghost(mut commands: Commands, cell_assets: Res<CellAssets>) {
    let ghost_pos = Position { x: 0, y: 0 };

    commands.spawn((
        HoverGhost,
        ghost_pos,
        Mesh2d(cell_assets.mesh.clone()),
        MeshMaterial2d(cell_assets.ghost_material.clone()),
        Transform::from_xyz(
            ghost_pos.x as f32 * CELL_WIDTH,
            ghost_pos.y as f32 * CELL_WIDTH,
            0.5,
        ),
        Visibility::Hidden,
    ));
}

fn update_hover_ghost(
    mouse_position_res: Res<MouseGridPosition>,
    mut hover_ghost_query: Query<(&mut Visibility, &mut Transform), With<HoverGhost>>,
) {
    let Ok((mut visibility, mut transform)) = hover_ghost_query.single_mut() else {
        return;
    };

    let Some(mouse_position) = mouse_position_res.cur else {
        *visibility = Visibility::Hidden;
        return;
    };

    *visibility = Visibility::Visible;
    transform.translation.x = mouse_position.x as f32 * CELL_WIDTH;
    transform.translation.y = mouse_position.y as f32 * CELL_WIDTH;
}

fn update_drawing_set(
    mut commands: Commands,
    mouse_position_res: Res<MouseGridPosition>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    draw_ghost_query: Query<(Entity, &Position), With<DrawingGhost>>,
    cell_assets: Res<CellAssets>,
) {
    if !buttons.pressed(MouseButton::Left) {
        // if mouse is not pressed and no CTRL key is pressed, spawn cells
        if !keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
            for (entity, pos) in draw_ghost_query.iter() {
                commands.trigger(SpawnCellEvent::new(pos.x, pos.y));
                commands.entity(entity).despawn();
            }
        }

        return;
    }

    let cur_pos = match mouse_position_res.cur {
        Some(pos) => pos,
        None => return,
    };

    let prev_pos = if buttons.just_pressed(MouseButton::Left) {
        cur_pos
    } else {
        mouse_position_res.prev.unwrap_or(cur_pos)
    };

    let existing: HashSet<Position> = draw_ghost_query.iter().map(|(_, pos)| *pos).collect();

    // Calculate deltas
    let mut dx = cur_pos.x - prev_pos.x;
    let mut dy = cur_pos.y - prev_pos.y;

    let incx = dx.signum();
    let incy = dy.signum();

    dx = dx.abs();
    dy = dy.abs();

    let (pdx, pdy, ddx, ddy, delta_slow, delta_fast) = if dx > dy {
        (incx, 0, incx, incy, dy, dx)
    } else {
        (0, incy, incx, incy, dx, dy)
    };

    let mut x = prev_pos.x;
    let mut y = prev_pos.y;
    let mut err = delta_fast / 2;

    // Draw the first pixel
    let pos = Position { x, y };
    if !existing.contains(&pos) {
        commands.spawn((
            DrawingGhost,
            pos,
            Mesh2d(cell_assets.mesh.clone()),
            MeshMaterial2d(cell_assets.ghost_material.clone()),
            Transform::from_xyz(x as f32 * CELL_WIDTH, y as f32 * CELL_WIDTH, 0.5),
        ));
    }

    for _ in 0..delta_fast {
        err -= delta_slow;
        if err < 0 {
            err += delta_fast;
            x += ddx;
            y += ddy;
        } else {
            x += pdx;
            y += pdy;
        }

        let pos = Position { x, y };
        if !existing.contains(&pos) {
            commands.spawn((
                DrawingGhost,
                pos,
                Mesh2d(cell_assets.mesh.clone()),
                MeshMaterial2d(cell_assets.ghost_material.clone()),
                Transform::from_xyz(x as f32 * CELL_WIDTH, y as f32 * CELL_WIDTH, 0.5),
            ));
        }
    }
}
