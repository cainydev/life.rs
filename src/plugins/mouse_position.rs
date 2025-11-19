use bevy::{prelude::*, window::PrimaryWindow};

use crate::simulation::coords::world_to_cell;

pub struct MousePositionPlugin;

impl Plugin for MousePositionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MouseGridPosition>();
        app.init_resource::<MousePixelPosition>();
        app.add_systems(PreUpdate, update_mouse_position);
    }
}

#[derive(Resource, Default)]
pub struct MouseGridPosition {
    pub prev: Option<IVec2>,
    pub cur: Option<IVec2>,
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
            mouse_grid_pos.cur = Some(world_to_cell(world_pos));
        } else {
            mouse_grid_pos.cur = None;
        }
    } else {
        mouse_grid_pos.cur = None;
    }
}
