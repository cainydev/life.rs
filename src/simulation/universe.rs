use bevy::math::I64Vec2;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use std::sync::{Arc, RwLock};

use crate::simulation::engine::{EngineMode, LifeEngine, create_engine};
use crate::simulation::stats_boards::StatsBoard;

pub struct UniversePlugin;

impl Plugin for UniversePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Universe>()
            // The step logic now initiates and polls tasks.
            .add_systems(Update, step_universe)
            // Separate system to handle input and trigger state changes.
            .add_systems(PreUpdate, handle_input);
    }
}

// --- Simplified Universe Resource ---

// Use a type alias for cleaner code
type SharedEngine = Arc<RwLock<Box<dyn LifeEngine>>>;

#[derive(Resource)]
pub struct Universe {
    // The single source of truth for the engine, shared between threads.
    engine: SharedEngine,

    // Stores the Task spawned for the background step. The task now returns () instead of Duration.
    step_task: Option<Task<()>>,

    // Config: How many steps to take per frame
    pub steps_per_frame: u64,
}

impl Default for Universe {
    fn default() -> Self {
        let engine = create_engine(EngineMode::ArenaLife);
        Self {
            // Initialize the engine wrapped in Arc<RwLock<...>>
            engine: Arc::new(RwLock::new(engine)),
            step_task: None,
            steps_per_frame: 1,
        }
    }
}

impl Universe {
    #[allow(unused)]
    pub fn read_engine(&self) -> std::sync::RwLockReadGuard<'_, Box<dyn LifeEngine>> {
        self.engine.read().unwrap()
    }

    #[allow(unused)]
    pub fn set_cell(&mut self, pos: I64Vec2, alive: bool) {
        if let Ok(mut engine) = self.engine.write() {
            engine.set_cell(pos, alive);
        }
    }

    pub fn add_cells(&mut self, cells: Vec<I64Vec2>) {
        if let Ok(mut engine) = self.engine.write() {
            engine.set_cells(&cells, true);
        }
    }

    pub fn clear(&mut self) {
        if let Ok(mut engine) = self.engine.write() {
            engine.clear();
        }
    }

    #[allow(unused)]
    pub fn import(&mut self, cells: Vec<I64Vec2>) {
        if let Ok(mut engine) = self.engine.write() {
            engine.import(&cells);
        }
    }

    pub fn switch_engine(&mut self, mode: EngineMode) {
        println!("Switching Engine to {:?}", mode);
        if let Ok(mut old_engine) = self.engine.write() {
            // 1. Export state
            let cells = old_engine.export();

            // 2. Create and import into the new engine
            let mut new_engine = create_engine(mode);
            new_engine.import(&cells);

            // 3. Swap the engine inside the lock
            *old_engine = new_engine;
        }
    }

    // Public API for view/stats remains clean, reading from the single source of truth
    pub fn draw_to_buffer(&self, rect: Rect, buffer: &mut [u8], width: usize, height: usize) {
        if let Ok(engine) = self.engine.read() {
            engine.draw_to_buffer(rect, buffer, width, height);
        }
    }

    pub fn population(&self) -> u64 {
        self.engine.read().map(|e| e.population()).unwrap_or(0)
    }

    pub fn engine_name(&self) -> String {
        self.engine
            .read()
            .map(|e| e.name().to_string())
            .unwrap_or_default()
    }
}

// --- Systems ---

fn step_universe(mut universe: ResMut<Universe>, mut stats: ResMut<StatsBoard>) {
    // 1. Check if a step is running and poll it
    if let Some(mut task) = universe.step_task.take() {
        if poll_task_once(&mut task).is_some() {
            // Task is complete: Update Stats (excluding step time)
            stats.insert("Engine", universe.engine_name()); // Read from the live engine

        // Task has been consumed by `task.take()`
        } else {
            // Task is still running: put it back
            universe.step_task = Some(task);
            return;
        }
    }

    // 2. Start a new step if no task is currently running/being polled
    if universe.step_task.is_none() {
        let shared_engine_ref = Arc::clone(&universe.engine);
        let steps = universe.steps_per_frame;

        let thread_pool = AsyncComputeTaskPool::get();

        let task = thread_pool.spawn(async move {
            if let Ok(mut engine) = shared_engine_ref.write() {
                engine.step(steps);
            }
        });

        universe.step_task = Some(task);
    }
}

// Handles key input and triggers state changes directly on the locked engine.
fn handle_input(mut universe: ResMut<Universe>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::KeyC) {
        universe.clear();
        println!("Universe cleared!");
    }

    let switch_mode = if keys.just_pressed(KeyCode::Digit1) {
        Some(EngineMode::ArenaLife)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(EngineMode::SparseLife)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(EngineMode::HashLife)
    } else {
        None
    };

    if let Some(mode) = switch_mode {
        // The switch happens synchronously on the main thread,
        // taking a brief write lock on the engine.
        universe.switch_engine(mode);
    }
}

// Standard Bevy boilerplate for polling tasks without blocking.
fn poll_task_once<T>(task: &mut Task<T>) -> Option<T> {
    let waker = noop_waker();
    let mut cx = std::task::Context::from_waker(&waker);
    match std::pin::Pin::new(task).poll(&mut cx) {
        std::task::Poll::Ready(output) => Some(output),
        std::task::Poll::Pending => None,
    }
}

unsafe fn noop_clone(_: *const ()) -> std::task::RawWaker {
    noop_raw_waker()
}
unsafe fn noop(_: *const ()) {}

fn noop_raw_waker() -> std::task::RawWaker {
    static VTABLE: std::task::RawWakerVTable =
        std::task::RawWakerVTable::new(noop_clone, noop, noop, noop);
    std::task::RawWaker::new(std::ptr::null(), &VTABLE)
}

fn noop_waker() -> std::task::Waker {
    unsafe { std::task::Waker::from_raw(noop_raw_waker()) }
}
