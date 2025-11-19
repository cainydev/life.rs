use bevy::{input::mouse::MouseWheel, prelude::*, window::PrimaryWindow};

pub struct CameraMovementPlugin;

impl Plugin for CameraMovementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraMovementSettings>()
            .init_resource::<CameraMovementState>()
            .add_systems(
                Update,
                (start_pan, pan_camera, zoom_camera.after(pan_camera)).in_set(CameraMovementSet),
            );
    }
}

#[derive(Resource, Default)]
struct CameraMovementState {
    panning: bool,
    last_cursor_world_pos: Option<Vec2>,
}

#[derive(Resource)]
pub struct CameraMovementSettings {
    pub zoom_sensitivity: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
}

impl Default for CameraMovementSettings {
    fn default() -> Self {
        Self {
            zoom_sensitivity: 0.1,
            min_zoom: 0.0,
            max_zoom: 1000.0, // f32::MAX ist oft zu extrem für Kameras
        }
    }
}

#[derive(SystemSet, Hash, Debug, Clone, PartialEq, Eq)]
pub struct CameraMovementSet;

fn start_pan(
    mut movement_state: ResMut<CameraMovementState>,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    if let Some(cursor_pos) = window.cursor_position() {
        if buttons.just_pressed(MouseButton::Right) {
            if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                movement_state.panning = true;
                movement_state.last_cursor_world_pos = Some(world_pos);
            }
        } else if buttons.just_released(MouseButton::Right) {
            movement_state.panning = false;
            movement_state.last_cursor_world_pos = None;
        }
    }
}

fn pan_camera(
    movement_state: Res<CameraMovementState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut camera_transform_query: Query<&mut Transform, With<Camera2d>>,
) {
    if !movement_state.panning {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((camera, camera_global_transform)) = camera_query.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_transform_query.single_mut() else {
        return;
    };

    if let (Some(cursor_pos), Some(last_cursor_world_pos)) = (
        window.cursor_position(),
        movement_state.last_cursor_world_pos,
    ) {
        if let Ok(current_cursor_world_pos) =
            camera.viewport_to_world_2d(camera_global_transform, cursor_pos)
        {
            let world_delta = last_cursor_world_pos - current_cursor_world_pos;
            camera_transform.translation.x += world_delta.x;
            camera_transform.translation.y += world_delta.y;
        }
    }
}

fn zoom_camera(
    mut scroll_events: MessageReader<MouseWheel>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut camera_transform_query: Query<&mut Transform, With<Camera2d>>,
    settings: Res<CameraMovementSettings>,
) {
    let Ok((camera, global_transform)) = camera_query.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_transform_query.single_mut() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };

    for ev in scroll_events.read() {
        let target_world_pos = window
            .cursor_position()
            .and_then(|cursor_pos| {
                camera
                    .viewport_to_world_2d(global_transform, cursor_pos)
                    .ok()
            })
            .unwrap_or(camera_transform.translation.xy());

        let zoom_factor = 1.0 - (ev.y * settings.zoom_sensitivity);
        let old_scale = camera_transform.scale.x;
        let new_scale = (old_scale * zoom_factor).clamp(settings.min_zoom, settings.max_zoom);

        if (new_scale - old_scale).abs() < f32::EPSILON {
            continue;
        }

        let scale_ratio = new_scale / old_scale;
        camera_transform.scale = Vec3::splat(new_scale); // X, Y und Z gleich skalieren

        let old_translation = camera_transform.translation.xy();
        let new_translation = target_world_pos + (old_translation - target_world_pos) * scale_ratio;

        camera_transform.translation.x = new_translation.x;
        camera_transform.translation.y = new_translation.y;
    }
}
