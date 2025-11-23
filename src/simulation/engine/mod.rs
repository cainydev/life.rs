use bevy::math::{I64Vec2, Rect};

use crate::simulation::engine::{
    arena_life::ArenaLife, hash_life::HashLife, sparse_life::SparseLife,
};

mod arena_life;
mod hash_life;
mod sparse_life;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineMode {
    ArenaLife,
    SparseLife,
    HashLife,
}

// 1. The Trait must be Object Safe.
// We cannot inherit 'Clone' directly because 'clone()' returns Self (Sized).
// We use a helper 'box_clone' instead.
pub trait LifeEngine: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn step(&mut self, steps: u64) -> u64;
    fn clear(&mut self);

    fn population(&self) -> u64;

    fn set_cell(&mut self, pos: I64Vec2, alive: bool);
    fn get_cell(&self, pos: I64Vec2) -> bool;

    fn set_cells(&mut self, coords: &[I64Vec2], alive: bool);

    fn import(&mut self, alive_cells: &[I64Vec2]);
    fn export(&self) -> Vec<I64Vec2>;

    fn draw_to_buffer(&self, world_rect: Rect, buffer: &mut [u8], width: usize, height: usize);

    // The Magic Method for cloning Box<dyn LifeEngine>
    fn box_clone(&self) -> Box<dyn LifeEngine>;
}

// 2. Implement Clone for the Boxed Trait
impl Clone for Box<dyn LifeEngine> {
    fn clone(&self) -> Box<dyn LifeEngine> {
        self.box_clone()
    }
}

// 3. Factory Function to create engines
pub fn create_engine(mode: EngineMode) -> Box<dyn LifeEngine> {
    match mode {
        EngineMode::ArenaLife => Box::new(ArenaLife::new()),
        EngineMode::SparseLife => Box::new(SparseLife::new()),
        EngineMode::HashLife => Box::new(HashLife::new()),
    }
}
