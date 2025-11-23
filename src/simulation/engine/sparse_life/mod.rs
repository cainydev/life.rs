use crate::simulation::engine::LifeEngine;
use bevy::math::{I64Vec2, Rect};
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};

const BLOCK_SIZE: usize = 64;

#[derive(Clone, Copy)]
struct Block {
    rows: [u64; BLOCK_SIZE],
}

impl Default for Block {
    fn default() -> Self {
        Self {
            rows: [0; BLOCK_SIZE],
        }
    }
}

#[derive(Clone)]
pub struct SparseLife {
    // Primary State
    blocks: FxHashMap<I64Vec2, Block>,
    active: FxHashSet<I64Vec2>,

    // Secondary State (Buffers for Double Buffering)
    next_blocks: FxHashMap<I64Vec2, Block>,
    next_active: FxHashSet<I64Vec2>,

    // Scratchpad for step coordination
    to_evaluate: FxHashSet<I64Vec2>,

    generation: u64,
}

impl SparseLife {
    pub fn new() -> Self {
        Self {
            blocks: FxHashMap::default(),
            active: FxHashSet::default(),
            next_blocks: FxHashMap::default(),
            next_active: FxHashSet::default(),
            to_evaluate: FxHashSet::default(),
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

    // Optimized: Unswitched loop to remove branches from the hot path
    fn evolve_block(
        current: &Block,
        n: Option<&Block>,
        s: Option<&Block>,
        w: Option<&Block>,
        e: Option<&Block>,
        nw: Option<&Block>,
        ne: Option<&Block>,
        sw: Option<&Block>,
        se: Option<&Block>,
    ) -> (Block, bool) {
        let mut next = Block::default();
        let mut alive = false;

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

                next.rows[$y_idx] = res;
                if res != 0 {
                    alive = true;
                }
            }};
        }

        #[inline(always)]
        fn bit_w(b: Option<&Block>, row: usize) -> u64 {
            b.map(|x| (x.rows[row] >> 63) & 1).unwrap_or(0)
        }
        #[inline(always)]
        fn bit_e(b: Option<&Block>, row: usize) -> u64 {
            b.map(|x| (x.rows[row] & 1) << 63).unwrap_or(0)
        }

        // --- 1. Top Row (Y=0) ---
        {
            let up = n.map(|b| b.rows[BLOCK_SIZE - 1]).unwrap_or(0);
            let center = current.rows[0];
            let down = current.rows[1];

            let w_u = bit_w(nw, BLOCK_SIZE - 1);
            let w_c = bit_w(w, 0);
            let w_d = bit_w(w, 1);
            let e_u = bit_e(ne, BLOCK_SIZE - 1);
            let e_c = bit_e(e, 0);
            let e_d = bit_e(e, 1);

            calc_row!(0, up, center, down, w_u, w_c, w_d, e_u, e_c, e_d);
        }

        // --- 2. Middle Rows (Y=1..63) ---
        for y in 1..BLOCK_SIZE - 1 {
            let up = current.rows[y - 1];
            let center = current.rows[y];
            let down = current.rows[y + 1];

            let w_u = bit_w(w, y - 1);
            let w_c = bit_w(w, y);
            let w_d = bit_w(w, y + 1);
            let e_u = bit_e(e, y - 1);
            let e_c = bit_e(e, y);
            let e_d = bit_e(e, y + 1);

            calc_row!(y, up, center, down, w_u, w_c, w_d, e_u, e_c, e_d);
        }

        // --- 3. Bottom Row (Y=63) ---
        {
            let up = current.rows[BLOCK_SIZE - 2];
            let center = current.rows[BLOCK_SIZE - 1];
            let down = s.map(|b| b.rows[0]).unwrap_or(0);

            let w_u = bit_w(w, BLOCK_SIZE - 2);
            let w_c = bit_w(w, BLOCK_SIZE - 1);
            let w_d = bit_w(sw, 0);
            let e_u = bit_e(e, BLOCK_SIZE - 2);
            let e_c = bit_e(e, BLOCK_SIZE - 1);
            let e_d = bit_e(se, 0);

            calc_row!(
                BLOCK_SIZE - 1,
                up,
                center,
                down,
                w_u,
                w_c,
                w_d,
                e_u,
                e_c,
                e_d
            );
        }
        (next, alive)
    }

    // --- Rendering Helpers ---

    /// Path A: Sparse Rendering (World Space -> Screen Space)
    /// Used when population is low. Iterates active blocks and draws rectangles.
    fn draw_sparse(&self, rect: Rect, buffer: &mut [u8], width: usize, height: usize, scale: f64) {
        // Clear buffer first (Essential, as we only draw "on" pixels)
        buffer.fill(0);

        let view_min_x = rect.min.x as f64;
        let view_min_y = rect.min.y as f64;
        let bs = BLOCK_SIZE as i64;
        let block_screen_size = bs as f64 * scale;

        // Iterate over BLOCKS that contain cells
        for (&chunk_pos, block) in &self.blocks {
            // Culling (Approximate AABB overlap check)
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

            // Iterate active cells in this block
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

                        // Draw the cell using the fixed rounding logic
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
                // FIX 1: Center Sampling + Floor for Y-axis
                let center_y = rect.min.y as f64 + ((screen_y + 0.5) * inv_scale);
                let global_y = center_y.floor() as i64;

                let mut current_chunk_idx = I64Vec2::new(i64::MAX, i64::MAX);
                let mut current_block: Option<&Block> = None;

                for (x, pixel) in pixel_row.iter_mut().enumerate() {
                    let screen_x = x as f64;
                    let center_x = rect.min.x as f64 + ((screen_x + 0.5) * inv_scale);
                    let global_x = center_x.floor() as i64;

                    let block_x = global_x.div_euclid(bs);
                    let block_y = global_y.div_euclid(bs);
                    let chunk_pos = I64Vec2::new(block_x, block_y);

                    if chunk_pos != current_chunk_idx {
                        current_chunk_idx = chunk_pos;
                        current_block = self.blocks.get(&chunk_pos);
                    }

                    *pixel = 0;

                    if let Some(block) = current_block {
                        if is_zoomed_in {
                            let local_x = global_x.rem_euclid(bs) as usize;
                            let local_y = global_y.rem_euclid(bs) as usize;

                            if (block.rows[local_y] >> local_x) & 1 == 1 {
                                *pixel = 255;
                            }
                        } else {
                            let base_x = block_x * bs;
                            let base_y = block_y * bs;

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
}

impl LifeEngine for SparseLife {
    fn id(&self) -> &str {
        "sparse-life"
    }

    fn name(&self) -> &str {
        "SparseLife"
    }

    fn population(&self) -> u64 {
        self.blocks
            .values()
            .map(|b| b.rows.iter().map(|r| r.count_ones() as u64).sum::<u64>())
            .sum()
    }

    fn set_cell(&mut self, pos: I64Vec2, alive: bool) {
        self.set_cells(&[pos], alive);
    }

    fn set_cells(&mut self, coords: &[I64Vec2], alive: bool) {
        for &pos in coords {
            let (chunk_pos, lx, ly) = Self::get_coords(pos.x, pos.y);
            let block = self.blocks.entry(chunk_pos).or_insert_with(Block::default);

            if alive {
                block.rows[ly] |= 1u64 << lx;
            } else {
                block.rows[ly] &= !(1u64 << lx);
            }

            // Mark block and neighbors as active
            for dy in -1..=1 {
                for dx in -1..=1 {
                    self.active.insert(chunk_pos + I64Vec2::new(dx, dy));
                }
            }
        }
    }

    fn get_cell(&self, pos: I64Vec2) -> bool {
        let (chunk_pos, lx, ly) = Self::get_coords(pos.x, pos.y);
        if let Some(block) = self.blocks.get(&chunk_pos) {
            (block.rows[ly] >> lx) & 1 == 1
        } else {
            false
        }
    }

    fn clear(&mut self) {
        self.blocks.clear();
        self.active.clear();
        self.next_blocks.clear();
        self.next_active.clear();
        self.to_evaluate.clear();
        self.generation = 0;
    }

    fn export(&self) -> Vec<I64Vec2> {
        let mut cells = Vec::new();
        for (pos, block) in &self.blocks {
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
            self.to_evaluate.clear();
            for &pos in &self.active {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        self.to_evaluate.insert(pos + I64Vec2::new(dx, dy));
                    }
                }
            }
            let eval_list: Vec<I64Vec2> = self.to_evaluate.iter().copied().collect();
            self.next_blocks.clear();
            self.next_active.clear();

            let results: Vec<(I64Vec2, Block)> = eval_list
                .par_iter()
                .filter_map(|&pos| {
                    let get_b = |dx, dy| self.blocks.get(&(pos + I64Vec2::new(dx, dy)));
                    let current = get_b(0, 0);

                    if current.is_none() {
                        let has_neighbor = (-1..=1).any(|dy| {
                            (-1..=1).any(|dx| {
                                (dx != 0 || dy != 0)
                                    && self.blocks.contains_key(&(pos + I64Vec2::new(dx, dy)))
                            })
                        });
                        if !has_neighbor {
                            return None;
                        }
                    }

                    let default = Block::default();
                    let curr_ref = current.unwrap_or(&default);

                    let (n, s, w, e, nw, ne, sw, se) = (
                        get_b(0, -1),
                        get_b(0, 1),
                        get_b(-1, 0),
                        get_b(1, 0),
                        get_b(-1, -1),
                        get_b(1, -1),
                        get_b(-1, 1),
                        get_b(1, 1),
                    );
                    let (next_block, is_alive) =
                        Self::evolve_block(curr_ref, n, s, w, e, nw, ne, sw, se);

                    if is_alive {
                        Some((pos, next_block))
                    } else {
                        None
                    }
                })
                .collect();

            for (pos, block) in results {
                self.next_blocks.insert(pos, block);
                self.next_active.insert(pos);
            }

            std::mem::swap(&mut self.blocks, &mut self.next_blocks);
            std::mem::swap(&mut self.active, &mut self.next_active);
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

        let is_sparse = self.population() < (total_pixels as u64 / 10);

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
