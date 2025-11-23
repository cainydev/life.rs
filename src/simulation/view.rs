use bevy::input::mouse::MouseWheel;
use bevy::math::{DVec2, I64Vec2, Vec2};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulationView>()
            .init_resource::<MouseWorldPosition>()
            .add_systems(Update, (update_view_transform, update_mouse_world_pos));
    }
}

#[derive(Resource)]
pub struct SimulationView {
    pub center: DVec2,
    pub zoom: f64,
}

impl Default for SimulationView {
    fn default() -> Self {
        Self {
            center: DVec2::ZERO,
            zoom: 50.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct MouseWorldPosition {
    pub world_pos: Option<DVec2>,
    pub grid_pos: Option<I64Vec2>,
}

fn update_view_transform(
    mut view: ResMut<SimulationView>,
    mut events: MessageReader<MouseWheel>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut cursor_moved: MessageReader<CursorMoved>,
    mut last_cursor_pos: Local<Option<Vec2>>,
    // Use the mouse world position resource
    mouse_world_pos_res: Res<MouseWorldPosition>,
) {
    const ZOOM_STEP_FACTOR: f64 = 1.1;

    if let Some(world_pos_before_zoom) = mouse_world_pos_res.world_pos {
        for ev in events.read() {
            let direction: f64 = ev.y.signum() as f64;

            let scale_factor: f64 = if direction > 0.0 {
                ZOOM_STEP_FACTOR
            } else if direction < 0.0 {
                1.0 / ZOOM_STEP_FACTOR
            } else {
                1.0
            };

            let old_zoom = view.zoom;

            view.zoom = (view.zoom * scale_factor).clamp(0.01, 500.0);
            let new_zoom = view.zoom;

            if new_zoom != old_zoom {
                let zoom_ratio = old_zoom / new_zoom;
                let offset_from_center = world_pos_before_zoom - view.center;
                view.center += offset_from_center * (1.0 - zoom_ratio);
            }
        }
    } else {
        for _ in events.read() {}
    }

    if let Some(current_pos) = cursor_moved.read().last().map(|e| e.position) {
        if let Some(prev_pos) = *last_cursor_pos {
            if buttons.pressed(MouseButton::Right) || keys.pressed(KeyCode::Space) {
                let screen_delta = current_pos - prev_pos;
                // Important: Y is inverted for World Space
                let world_delta =
                    DVec2::new(screen_delta.x as f64, -screen_delta.y as f64) / view.zoom;
                view.center -= world_delta;
            }
        }
        *last_cursor_pos = Some(current_pos);
    }
}

fn update_mouse_world_pos(
    window: Query<&Window, With<PrimaryWindow>>,
    view: Res<SimulationView>,
    mut mouse_res: ResMut<MouseWorldPosition>,
) {
    let Ok(window) = window.single() else { return };

    if let Some(screen_pos) = window.cursor_position() {
        // 1. Center of screen (Logical Pixels)
        let center_x = window.width() / 2.0;
        let center_y = window.height() / 2.0;

        // 2. Offset from center (Logical Pixels)
        // Note: screen_pos.y is Top-Down. We invert it to align with World Up.
        let dx = (screen_pos.x - center_x) as f64;
        let dy = ((window.height() - screen_pos.y) - center_y) as f64;

        // 3. Convert to World Units
        // We match LayerViewport logic: Center + (LogicalOffset / Zoom)
        let world_x = view.center.x + (dx / view.zoom);
        let world_y = view.center.y + (dy / view.zoom);
        let world_pos = DVec2::new(world_x, world_y);

        mouse_res.world_pos = Some(world_pos);

        // 4. Convert to Grid Units (1 World Unit = 1 Cell)
        // We simply floor the float values to get integer grid coordinates.
        mouse_res.grid_pos = Some(I64Vec2::new(
            world_pos.x.floor() as i64,
            world_pos.y.floor() as i64,
        ));
    } else {
        mouse_res.world_pos = None;
        mouse_res.grid_pos = None;
    }
}
