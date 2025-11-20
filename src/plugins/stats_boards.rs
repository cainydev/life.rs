use std::{collections::BTreeMap, fmt::Display};

use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct StatsBoard {
    data: BTreeMap<String, String>,
}

impl StatsBoard {
    /// Insert or update a stat.
    /// Accepts any value that implements Display (f32, int, strings, etc.)
    pub fn insert<V: Display>(&mut self, key: &str, value: V) {
        self.data.insert(key.to_string(), value.to_string());
    }

    /// Remove a specific stat
    pub fn remove(&mut self, key: &str) {
        self.data.remove(key);
    }

    /// Clear all stats
    pub fn clear(&mut self) {
        self.data.clear();
    }
}

pub struct StatsBoardPlugin;

impl Plugin for StatsBoardPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StatsBoard>()
            .add_systems(Startup, setup_stats_ui)
            .add_systems(Update, update_stats_display);
    }
}

#[derive(Component)]
struct StatsText;

fn setup_stats_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                left: Val::Px(10.0),
                padding: UiRect::all(Val::Px(10.0)),
                // Semi-transparent background for readability
                ..default()
            },
            BackgroundColor(Color::BLACK.with_alpha(0.7)),
            GlobalZIndex(100), // Ensure it sits on top of everything
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Initializing Stats..."),
                TextFont {
                    font,
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                StatsText,
            ));
        });
}

fn update_stats_display(board: Res<StatsBoard>, mut query: Query<&mut Text, With<StatsText>>) {
    if board.is_changed() {
        for mut text in &mut query {
            if board.data.is_empty() {
                **text = "No Stats".to_string();
            } else {
                // Build a single string: "Key: Value\nKey2: Value2"
                let mut output = String::new();
                for (key, value) in &board.data {
                    use std::fmt::Write; // Allow write! macro on String
                    let _ = writeln!(output, "{}: {}", key, value);
                }
                // Update the Text component
                **text = output;
            }
        }
    }
}

fn random_stats_system(time: Res<Time>, mut stats: ResMut<StatsBoard>) {
    stats.insert("Game Version", "0.1.0");
    stats.insert("Time Elapsed", format!("{:.2}", time.elapsed_secs()));
}
