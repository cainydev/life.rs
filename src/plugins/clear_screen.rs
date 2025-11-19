use crate::simulation::universe::Universe;
use bevy::prelude::*; // Importiere das neue Universe

pub struct ClearScreenPlugin;

impl Plugin for ClearScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, clear_screen);
    }
}

fn clear_screen(mut universe: ResMut<Universe>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::KeyC) {
        universe.chunks.clear();
        println!("Screen cleared (Chunks dropped)");
    }
}
