use bevy::platform::sync::Mutex;
use bevy::prelude::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

/// The Bevy resource that holds our thread-safe RNG.
/// The `Mutex` is necessary for thread-safety (`Sync`).
#[derive(Resource)]
pub struct GlobalRng(pub Mutex<StdRng>);

pub struct SeededRngPlugin {
    pub seed: [u8; 32],
}

impl SeededRngPlugin {
    pub fn new(seed: [u8; 32]) -> Self {
        Self { seed }
    }
}

impl Plugin for SeededRngPlugin {
    fn build(&self, app: &mut App) {
        let rng = StdRng::from_seed(self.seed);
        app.insert_resource(GlobalRng(Mutex::new(rng)));
    }
}
