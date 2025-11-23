use crate::simulation::engine::LifeEngine;
use bevy::math::{I64Vec2, Rect};
use rayon::prelude::*;
use rustc_hash::FxHashMap;
use thunderdome::{Arena, Index};

const BLOCK_SIZE: usize = 64;

const N: usize = 0;
const S: usize = 1;
const W: usize = 2;
const E: usize = 3;
const NW: usize = 4;
const NE: usize = 5;
const SW: usize = 6;
const SE: usize = 7;

#[derive(Clone, Copy)]
struct Block {
    rows: [u64; BLOCK_SIZE],
    // Cache the Index of neighbors.
    neighbors: [Option<Index>; 8],
    alive: bool,
}

impl Default for Block {
    fn default() -> Self {
        Self {
            rows: [0; BLOCK_SIZE],
            neighbors: [None; 8],
            alive: false,
        }
    }
}

#[derive(Clone)]
pub struct ArenaLife {
    // The Data Store
    arena: Arena<Block>,
    // The Spatial Map
    lookup: FxHashMap<I64Vec2, Index>,

    // Scratchpads
    active_indices: Vec<(I64Vec2, Index)>,
    growth_requests: Vec<I64Vec2>,
    update_buffer: Vec<(Index, [u64; BLOCK_SIZE], bool)>,

    generation: u64,
}

impl ArenaLife {
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
            lookup: FxHashMap::default(),
            active_indices: Vec::new(),
            growth_requests: Vec::new(),
            update_buffer: Vec::new(),
            generation: 0,
        }
    }

    #[inline]
    fn get_coords(x: i64, y: i64) -> (I64Vec2, usize, usize) {
        let block_x = x.div_euclid(BLOCK_SIZE as i64);
        let block_y = y.div_euclid(BLOCK_SIZE as i64);
        let local_x = x.rem_euclid(BLOCK_SIZE as i64) as usize;
        let local_y = y.rem_euclid(BLOCK_SIZE as i64) as usize;
        (I64Vec2::new(block_x, block_y), local_x, local_y)
    }

    fn link(&mut self, pos: I64Vec2, idx: Index) {
        let offsets = [
            (0, -1, N, S),
            (0, 1, S, N),
            (-1, 0, W, E),
            (1, 0, E, W),
            (-1, -1, NW, SE),
            (1, -1, NE, SW),
            (-1, 1, SW, NE),
            (1, 1, SE, NW),
        ];

        for &(dx, dy, dir, opp_dir) in &offsets {
            let neighbor_pos = pos + I64Vec2::new(dx, dy);
            if let Some(&n_idx) = self.lookup.get(&neighbor_pos) {
                self.arena[idx].neighbors[dir] = Some(n_idx);
                self.arena[n_idx].neighbors[opp_dir] = Some(idx);
            }
        }
    }

    fn spawn_block(&mut self, pos: I64Vec2) -> Index {
        if let Some(&idx) = self.lookup.get(&pos) {
            idx
        } else {
            let idx = self.arena.insert(Block::default());
            self.lookup.insert(pos, idx);
            self.link(pos, idx);
            idx
        }
    }

    // --- Rendering Helpers ---

    /// Path A: Sparse Rendering (World Space -> Screen Space)
    /// Used when population is low. Iterates active blocks and draws rectangles.
    fn draw_sparse(&self, rect: Rect, buffer: &mut [u8], width: usize, height: usize, scale: f64) {
        // Clear buffer first (memset optimized)
        buffer.fill(0);

        let view_min_x = rect.min.x as f64;
        let view_min_y = rect.min.y as f64;
        let bs = BLOCK_SIZE as i64;
        let block_screen_size = bs as f64 * scale;

        for (chunk_pos, &block_idx) in &self.lookup {
            let block = &self.arena[block_idx];
            if !block.alive {
                continue;
            }

            // Culling
            let block_world_x = chunk_pos.x * bs;
            let block_world_y = chunk_pos.y * bs;
            let screen_block_x = (block_world_x as f64 - view_min_x) * scale;
            let screen_block_y = (block_world_y as f64 - view_min_y) * scale;

            if screen_block_x > width as f64
                || screen_block_x + block_screen_size < 0.0
                || screen_block_y > height as f64
                || screen_block_y + block_screen_size < 0.0
            {
                continue;
            }

            for ly in 0..BLOCK_SIZE {
                let row = block.rows[ly];
                if row == 0 {
                    continue;
                }

                let world_y = (block_world_y + ly as i64) as f64;
                let sy = (world_y - view_min_y) * scale;

                for lx in 0..BLOCK_SIZE {
                    if (row >> lx) & 1 == 1 {
                        let world_x = (block_world_x + lx as i64) as f64;
                        let sx = (world_x - view_min_x) * scale;
                        self.fill_rect_safe(buffer, width, height, sx, sy, scale);
                    }
                }
            }
        }
    }

    /// Path B: Dense Rendering (Screen Space -> World Space)
    /// Used when population is high. Parallel iterates pixels and raycasts to grid.
    fn draw_dense(&self, rect: Rect, buffer: &mut [u8], width: usize, scale: f64) {
        let inv_scale = 1.0 / scale;
        let is_zoomed_in = scale >= 1.0;
        let bs = BLOCK_SIZE as i64;

        buffer
            .par_chunks_exact_mut(width)
            .enumerate()
            .for_each(|(y, pixel_row)| {
                let screen_y = y as f64;
                // FIX: Center Sampling + Floor
                let center_y = rect.min.y as f64 + ((screen_y + 0.5) * inv_scale);
                let global_y = center_y.floor() as i64;

                let mut current_chunk_idx = I64Vec2::new(i64::MAX, i64::MAX);
                let mut current_block: Option<&Block> = None;

                for (x, pixel) in pixel_row.iter_mut().enumerate() {
                    let screen_x = x as f64;
                    // FIX: Center Sampling + Floor
                    let center_x = rect.min.x as f64 + ((screen_x + 0.5) * inv_scale);
                    let global_x = center_x.floor() as i64;

                    // FIX: Euclidean Division ensures correct block index for negative coords
                    let block_x = global_x.div_euclid(bs);
                    let block_y = global_y.div_euclid(bs);
                    let chunk_pos = I64Vec2::new(block_x, block_y);

                    if chunk_pos != current_chunk_idx {
                        current_chunk_idx = chunk_pos;
                        current_block = self.lookup.get(&chunk_pos).map(|&idx| &self.arena[idx]);
                    }

                    *pixel = 0;

                    if let Some(block) = current_block {
                        if !block.alive {
                            continue;
                        }

                        if is_zoomed_in {
                            // Point Sampling
                            // FIX: Euclidean Remainder guarantees local_x is 0..63
                            let local_x = global_x.rem_euclid(bs) as usize;
                            let local_y = global_y.rem_euclid(bs) as usize;

                            if (block.rows[local_y] >> local_x) & 1 == 1 {
                                *pixel = 255;
                            }
                        } else {
                            // Area Sampling
                            let base_x = block_x * bs;
                            let base_y = block_y * bs;

                            // Calculate area relative to pixel center
                            let world_x_start = center_x - (0.5 * inv_scale);
                            let world_x_end = center_x + (0.5 * inv_scale);
                            let world_y_start = center_y - (0.5 * inv_scale);
                            let world_y_end = center_y + (0.5 * inv_scale);

                            let lx_start = ((world_x_start - base_x as f64).floor() as i64)
                                .clamp(0, 63) as usize;
                            let lx_end =
                                ((world_x_end - base_x as f64).ceil() as i64).clamp(1, 64) as usize;
                            let ly_start = ((world_y_start - base_y as f64).floor() as i64)
                                .clamp(0, 63) as usize;
                            let ly_end =
                                ((world_y_end - base_y as f64).ceil() as i64).clamp(1, 64) as usize;

                            let range_w = lx_end - lx_start;
                            if range_w > 0 && ly_end > ly_start {
                                let mask_bits = if range_w >= 64 {
                                    !0u64
                                } else {
                                    (1u64 << range_w) - 1
                                };
                                let row_mask = mask_bits << lx_start;

                                for r in ly_start..ly_end {
                                    if (block.rows[r] & row_mask) != 0 {
                                        *pixel = 255;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            });
    }

    /// Safe rectangle filler using rounding to avoid 'fat' blocks
    fn fill_rect_safe(
        &self,
        buffer: &mut [u8],
        width: usize,
        height: usize,
        x: f64,
        y: f64,
        size: f64,
    ) {
        let effective_size = size.max(1.0);

        // FIX: Rounding instead of Floor/Ceil prevents drift and overshoot
        let start_x = x.round() as isize;
        let start_y = y.round() as isize;
        let end_x = (x + effective_size).round() as isize;
        let end_y = (y + effective_size).round() as isize;

        let sx = start_x.max(0).min(width as isize) as usize;
        let sy = start_y.max(0).min(height as isize) as usize;
        let ex = end_x.max(0).min(width as isize) as usize;
        let ey = end_y.max(0).min(height as isize) as usize;

        if sx >= ex || sy >= ey {
            return;
        }

        for row in sy..ey {
            let offset = row * width;
            buffer[offset + sx..offset + ex].fill(255);
        }
    }

    fn evolve_block_internal(
        arena: &Arena<Block>,
        current_idx: Index,
    ) -> ([u64; BLOCK_SIZE], bool, u8) {
        let current = &arena[current_idx];
        let mut next_rows = [0u64; BLOCK_SIZE];
        let mut is_alive = false;
        let mut growth_flags: u8 = 0;

        macro_rules! calc_row {
            ($y_idx:expr, $up:expr, $center:expr, $down:expr, $w_bit_u:expr, $w_bit_c:expr, $w_bit_d:expr, $e_bit_u:expr, $e_bit_c:expr, $e_bit_d:expr) => {{
                let l_up = ($up << 1) | $w_bit_u;
                let r_up = ($up >> 1) | $e_bit_u;
                let l_curr = ($center << 1) | $w_bit_c;
                let r_curr = ($center >> 1) | $e_bit_c;
                let l_down = ($down << 1) | $w_bit_d;
                let r_down = ($down >> 1) | $e_bit_d;

                let mut s0 = 0u64;
                let mut s1 = 0u64;
                let mut s2 = 0u64;

                for x in [l_up, $up, r_up, l_curr, r_curr, l_down, $down, r_down] {
                    let c0 = s0 & x;
                    s0 ^= x;
                    let c1 = s1 & c0;
                    s1 ^= c0;
                    s2 |= c1;
                }

                let res = (s1 & !s2) & ($center | s0);
                next_rows[$y_idx] = res;
                if res != 0 {
                    is_alive = true;
                }
            }};
        }

        let get_row = |dir: usize, row: usize| -> u64 {
            match current.neighbors[dir] {
                Some(idx) => arena[idx].rows[row],
                None => 0,
            }
        };

        let bit_w = |dir: usize, row: usize| -> u64 {
            match current.neighbors[dir] {
                Some(idx) => (arena[idx].rows[row] >> 63) & 1,
                None => 0,
            }
        };

        let bit_e = |dir: usize, row: usize| -> u64 {
            match current.neighbors[dir] {
                Some(idx) => (arena[idx].rows[row] & 1) << 63,
                None => 0,
            }
        };

        {
            let up = get_row(N, BLOCK_SIZE - 1);
            let center = current.rows[0];
            let down = current.rows[1];
            if center != 0 && current.neighbors[N].is_none() {
                growth_flags |= 1 << N;
            }
            calc_row!(
                0,
                up,
                center,
                down,
                bit_w(NW, BLOCK_SIZE - 1),
                bit_w(W, 0),
                bit_w(W, 1),
                bit_e(NE, BLOCK_SIZE - 1),
                bit_e(E, 0),
                bit_e(E, 1)
            );
        }

        for y in 1..BLOCK_SIZE - 1 {
            let up = current.rows[y - 1];
            let center = current.rows[y];
            let down = current.rows[y + 1];
            if up | center | down == 0 {
                continue;
            }
            calc_row!(
                y,
                up,
                center,
                down,
                bit_w(W, y - 1),
                bit_w(W, y),
                bit_w(W, y + 1),
                bit_e(E, y - 1),
                bit_e(E, y),
                bit_e(E, y + 1)
            );
        }

        {
            let up = current.rows[BLOCK_SIZE - 2];
            let center = current.rows[BLOCK_SIZE - 1];
            let down = get_row(S, 0);
            if center != 0 && current.neighbors[S].is_none() {
                growth_flags |= 1 << S;
            }
            calc_row!(
                BLOCK_SIZE - 1,
                up,
                center,
                down,
                bit_w(W, BLOCK_SIZE - 2),
                bit_w(W, BLOCK_SIZE - 1),
                bit_w(SW, 0),
                bit_e(E, BLOCK_SIZE - 2),
                bit_e(E, BLOCK_SIZE - 1),
                bit_e(SE, 0)
            );
        }

        let mut all_or = 0u64;
        for r in current.rows {
            all_or |= r;
        }

        if (all_or >> 63) != 0 && current.neighbors[W].is_none() {
            growth_flags |= 1 << W;
        }
        if (all_or & 1) != 0 && current.neighbors[E].is_none() {
            growth_flags |= 1 << E;
        }
        if (current.rows[0] >> 63) & 1 == 1 && current.neighbors[NW].is_none() {
            growth_flags |= 1 << NW;
        }
        if (current.rows[0] & 1) == 1 && current.neighbors[NE].is_none() {
            growth_flags |= 1 << NE;
        }
        if (current.rows[BLOCK_SIZE - 1] >> 63) & 1 == 1 && current.neighbors[SW].is_none() {
            growth_flags |= 1 << SW;
        }
        if (current.rows[BLOCK_SIZE - 1] & 1) == 1 && current.neighbors[SE].is_none() {
            growth_flags |= 1 << SE;
        }

        (next_rows, is_alive, growth_flags)
    }
}

impl LifeEngine for ArenaLife {
    fn id(&self) -> &str {
        "arena-life"
    }

    fn name(&self) -> &str {
        "ArenaLife"
    }

    fn population(&self) -> u64 {
        self.arena
            .iter()
            .map(|(_, b)| b.rows.iter().map(|r| r.count_ones() as u64).sum::<u64>())
            .sum()
    }

    fn set_cell(&mut self, pos: I64Vec2, alive: bool) {
        self.set_cells(&[pos], alive);
    }

    fn set_cells(&mut self, coords: &[I64Vec2], alive: bool) {
        for &pos in coords {
            let (chunk_pos, lx, ly) = Self::get_coords(pos.x, pos.y);
            let idx = self.spawn_block(chunk_pos);
            let block = &mut self.arena[idx];
            if alive {
                block.rows[ly] |= 1u64 << lx;
                block.alive = true;
            } else {
                block.rows[ly] &= !(1u64 << lx);
            }
        }
    }

    fn get_cell(&self, pos: I64Vec2) -> bool {
        let (chunk_pos, lx, ly) = Self::get_coords(pos.x, pos.y);
        if let Some(&idx) = self.lookup.get(&chunk_pos) {
            (self.arena[idx].rows[ly] >> lx) & 1 == 1
        } else {
            false
        }
    }

    fn clear(&mut self) {
        self.arena.clear();
        self.lookup.clear();
        self.active_indices.clear();
        self.generation = 0;
    }

    fn export(&self) -> Vec<I64Vec2> {
        let mut cells = Vec::new();
        for (pos, &idx) in &self.lookup {
            let block = &self.arena[idx];
            if !block.alive {
                continue;
            }
            let base_x = pos.x * BLOCK_SIZE as i64;
            let base_y = pos.y * BLOCK_SIZE as i64;
            for y in 0..BLOCK_SIZE {
                let row = block.rows[y];
                if row == 0 {
                    continue;
                }
                for x in 0..BLOCK_SIZE {
                    if (row >> x) & 1 == 1 {
                        cells.push(I64Vec2::new(base_x + x as i64, base_y + y as i64));
                    }
                }
            }
        }
        cells
    }

    fn import(&mut self, alive_cells: &[I64Vec2]) {
        self.clear();
        self.set_cells(alive_cells, true);
    }

    fn step(&mut self, steps: u64) -> u64 {
        for _ in 0..steps {
            self.active_indices.clear();
            self.active_indices
                .extend(self.lookup.iter().map(|(p, i)| (*p, *i)));
            self.growth_requests.clear();
            self.update_buffer.clear();

            let arena_ref = &self.arena;
            let results: Vec<_> = self
                .active_indices
                .par_iter()
                .map(|&(pos, idx)| {
                    let (next_rows, alive, growth) = Self::evolve_block_internal(arena_ref, idx);
                    (idx, pos, next_rows, alive, growth)
                })
                .collect();

            for (idx, pos, next_rows, alive, growth_flags) in results {
                self.update_buffer.push((idx, next_rows, alive));
                if growth_flags != 0 {
                    if growth_flags & (1 << N) != 0 {
                        self.growth_requests.push(pos + I64Vec2::new(0, -1));
                    }
                    if growth_flags & (1 << S) != 0 {
                        self.growth_requests.push(pos + I64Vec2::new(0, 1));
                    }
                    if growth_flags & (1 << W) != 0 {
                        self.growth_requests.push(pos + I64Vec2::new(-1, 0));
                    }
                    if growth_flags & (1 << E) != 0 {
                        self.growth_requests.push(pos + I64Vec2::new(1, 0));
                    }
                    if growth_flags & (1 << NW) != 0 {
                        self.growth_requests.push(pos + I64Vec2::new(-1, -1));
                    }
                    if growth_flags & (1 << NE) != 0 {
                        self.growth_requests.push(pos + I64Vec2::new(1, -1));
                    }
                    if growth_flags & (1 << SW) != 0 {
                        self.growth_requests.push(pos + I64Vec2::new(-1, 1));
                    }
                    if growth_flags & (1 << SE) != 0 {
                        self.growth_requests.push(pos + I64Vec2::new(1, 1));
                    }
                }
            }

            for (idx, rows, alive) in self.update_buffer.drain(..) {
                let block = &mut self.arena[idx];
                block.rows = rows;
                block.alive = alive;
            }

            self.growth_requests
                .sort_unstable_by(|a, b| a.x.cmp(&b.x).then(a.y.cmp(&b.y)));
            self.growth_requests.dedup();
            let mut local_requests = std::mem::take(&mut self.growth_requests);
            for pos in local_requests.drain(..) {
                self.spawn_block(pos);
            }
            self.growth_requests = local_requests;
            self.generation += 1;
        }
        steps
    }

    fn draw_to_buffer(&self, rect: Rect, buffer: &mut [u8], width: usize, height: usize) {
        let scale = width as f64 / rect.width() as f64;

        if scale <= 0.0001 || scale.is_infinite() || scale.is_nan() {
            return;
        }

        let total_pixels = width * height;
        let is_sparse = self.population() < (total_pixels as u64 / 10) || scale > 0.5;

        if is_sparse {
            self.draw_sparse(rect, buffer, width, height, scale);
        } else {
            self.draw_dense(rect, buffer, width, scale);
        }
    }

    fn box_clone(&self) -> Box<dyn LifeEngine> {
        Box::new(self.clone())
    }
}
