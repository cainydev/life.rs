pub mod hashlife;

use bevy::math::Rect;

pub trait LifeEngine: Send + Sync {
    fn step(&mut self, steps: u64) -> u64;
    fn name(&self) -> &str;
    fn set_cell(&mut self, x: i64, y: i64, alive: bool);
    fn get_cell(&self, x: i64, y: i64) -> bool;
    fn clear(&mut self);
    fn population(&self) -> u64;

    fn export(&self) -> Vec<(i64, i64)>;
    fn import(&mut self, alive_cells: Vec<(i64, i64)>);

    fn draw_to_buffer(&self, world_rect: Rect, buffer: &mut [u8], width: usize, height: usize);
}
