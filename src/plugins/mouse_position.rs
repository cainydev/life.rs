use bevy::{prelude::*, window::PrimaryWindow};

use crate::{CELL_WIDTH, Position};

pub struct MousePositionPlugin;

impl Plugin for MousePositionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MouseGridPosition>();
        app.init_resource::<MousePixelPosition>();
        app.add_systems(FixedUpdate, update_mouse_position);
    }
}

#[derive(Resource, Default)]
pub struct MouseGridPosition {
    pub prev: Option<Position>,
    pub cur: Option<Position>,
}

#[derive(Resource, Default)]
pub struct MousePixelPosition {
    pub prev: Option<Vec2>,
    pub cur: Option<Vec2>,
}

pub fn update_mouse_position(
    mut mouse_grid_pos: ResMut<MouseGridPosition>,
    mut mouse_pixel_pos: ResMut<MousePixelPosition>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) {
    let Ok(window) = window_query.single() else {
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    if let Some(screen_pos) = window.cursor_position() {
        mouse_grid_pos.prev = mouse_grid_pos.cur;
        mouse_pixel_pos.prev = mouse_pixel_pos.cur;
        mouse_pixel_pos.cur = Some(screen_pos);

        if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, screen_pos) {
            let grid_x = (world_pos.x / CELL_WIDTH).round() as i32;
            let grid_y = (world_pos.y / CELL_WIDTH).round() as i32;

            mouse_grid_pos.cur = Some(Position {
                x: grid_x,
                y: grid_y,
            });
        } else {
            mouse_grid_pos.cur = None;
        }
    } else {
        mouse_grid_pos.cur = None;
    }
}
