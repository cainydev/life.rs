use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

// REMOVED: use crate::simulation::coords::world_to_cell;

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
    pub center: Vec2,
    pub zoom: f32,
}

impl Default for SimulationView {
    fn default() -> Self {
        Self {
            center: Vec2::ZERO,
            zoom: 50.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct MouseWorldPosition {
    pub world_pos: Option<Vec2>,
    pub grid_pos: Option<IVec2>,
}

fn update_view_transform(
    mut view: ResMut<SimulationView>,
    mut events: MessageReader<MouseWheel>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut cursor_moved: MessageReader<CursorMoved>,
    mut last_cursor_pos: Local<Option<Vec2>>,
) {
    // Zoom
    for ev in events.read() {
        let zoom_sensitivity = 0.1;
        let scale_factor = 1.0 + (ev.y * zoom_sensitivity);
        view.zoom = (view.zoom * scale_factor).clamp(0.5, 200.0);
    }

    // Pan
    if let Some(current_pos) = cursor_moved.read().last().map(|e| e.position) {
        if let Some(prev_pos) = *last_cursor_pos {
            if buttons.pressed(MouseButton::Right) || keys.pressed(KeyCode::Space) {
                let screen_delta = current_pos - prev_pos;
                // Important: Y is inverted for World Space
                let world_delta = Vec2::new(screen_delta.x, -screen_delta.y) / view.zoom;
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
        // --- Coordinate Space Logic ---
        // 1. Center of screen (Logical Pixels)
        let center_x = window.width() / 2.0;
        let center_y = window.height() / 2.0;

        // 2. Offset from center (Logical Pixels)
        // Note: screen_pos.y is Top-Down. We invert it to align with World Up.
        let dx = screen_pos.x - center_x;
        let dy = (window.height() - screen_pos.y) - center_y;

        // 3. Convert to World Units
        // We match LayerViewport logic: Center + (LogicalOffset / Zoom)
        let world_x = view.center.x + (dx / view.zoom);
        let world_y = view.center.y + (dy / view.zoom);
        let world_pos = Vec2::new(world_x, world_y);

        mouse_res.world_pos = Some(world_pos);

        // 4. Convert to Grid Units (1 World Unit = 1 Cell)
        // We simply floor the float values to get integer grid coordinates.
        mouse_res.grid_pos = Some(IVec2::new(
            world_pos.x.floor() as i32,
            world_pos.y.floor() as i32,
        ));
    } else {
        mouse_res.world_pos = None;
        mouse_res.grid_pos = None;
    }
}
