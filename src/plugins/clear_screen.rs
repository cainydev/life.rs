use bevy::prelude::*;

use crate::Cell;

pub struct ClearScreenPlugin;

impl Plugin for ClearScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, clear_screen);
    }
}

fn clear_screen(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    cell_query: Query<Entity, With<Cell>>,
) {
    if keys.just_pressed(KeyCode::KeyC) {
        for c in cell_query.iter() {
            commands.entity(c).despawn();
        }
    }
}
