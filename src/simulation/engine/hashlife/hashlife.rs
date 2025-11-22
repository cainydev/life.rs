use crate::simulation::engine::LifeEngine;

use super::cache::HashlifeCache;
use super::node::{Node, NodeData};
use bevy::math::Rect;
use std::sync::Arc;

pub struct Hashlife {
    cache: HashlifeCache,
    root: Arc<Node>,
    generation: u64,
    origin_x: i64,
    origin_y: i64,
}

impl Hashlife {
    pub fn new() -> Self {
        let mut cache = HashlifeCache::new();
        let root = cache.empty_node(4);

        Hashlife {
            cache,
            root,
            generation: 0,
            origin_x: 0,
            origin_y: 0,
        }
    }

    // [Helper] Checks if the active population is safely contained in the center
    // of the current node. Returns true if the "outer ring" is empty.
    fn is_padded(&self) -> bool {
        match &self.root.data {
            NodeData::Branch { nw, ne, sw, se, .. } => {
                // We want to verify that all population is inside the "inner core".
                // The inner core consists of:
                // NW's SE quadrant
                // NE's SW quadrant
                // SW's NE quadrant
                // SE's NW quadrant

                // If the root population equals the sum of these inner quadrants,
                // then the outer ring (padding) is completely empty.
                let inner_pop = self.get_sub_pop(nw, "se")
                    + self.get_sub_pop(ne, "sw")
                    + self.get_sub_pop(sw, "ne")
                    + self.get_sub_pop(se, "nw");

                self.root.population == inner_pop
            }
            // Leaves or small nodes are considered not padded (require expansion)
            // or safe enough to just run. For Level < 4, we usually force expansion.
            _ => false,
        }
    }

    // Helper to get population of a specific quadrant of a child node
    fn get_sub_pop(&self, node: &Arc<Node>, quadrant: &str) -> u64 {
        match &node.data {
            NodeData::Branch { nw, ne, sw, se, .. } => match quadrant {
                "nw" => nw.population,
                "ne" => ne.population,
                "sw" => sw.population,
                "se" => se.population,
                _ => 0,
            },
            _ => 0,
        }
    }

    /// Expands the universe to ensure it covers the given World Coordinates.
    fn expand_to_fit(&mut self, x: i64, y: i64) {
        for _ in 0..20 {
            let size = 1u64 << self.root.level();
            let rel_x = x - self.origin_x;
            let rel_y = y - self.origin_y;

            if rel_x >= 0 && rel_y >= 0 && rel_x < size as i64 && rel_y < size as i64 {
                return;
            }
            self.expand();
        }
    }

    /// Wraps the current root in empty nodes to center it.
    fn expand(&mut self) {
        match &self.root.data {
            NodeData::Branch {
                nw,
                ne,
                sw,
                se,
                level,
            } => {
                let empty = self.cache.empty_node(level - 1);
                let shift = 1i64 << (level - 1);
                self.origin_x -= shift;
                self.origin_y -= shift;

                let new_nw =
                    self.cache
                        .join(empty.clone(), empty.clone(), empty.clone(), nw.clone());
                let new_ne =
                    self.cache
                        .join(empty.clone(), empty.clone(), ne.clone(), empty.clone());
                let new_sw =
                    self.cache
                        .join(empty.clone(), sw.clone(), empty.clone(), empty.clone());
                let new_se =
                    self.cache
                        .join(se.clone(), empty.clone(), empty.clone(), empty.clone());

                self.root = self.cache.join(new_nw, new_ne, new_sw, new_se);
            }
            NodeData::Leaf(_) => {
                let empty = self.cache.empty_node(self.root.level());
                self.root = self.cache.join(
                    self.root.clone(),
                    empty.clone(),
                    empty.clone(),
                    empty.clone(),
                );
            }
        }
    }

    fn recursive_set(
        &mut self,
        node: Arc<Node>,
        size: u64,
        x: u64,
        y: u64,
        alive: bool,
    ) -> Arc<Node> {
        if let NodeData::Leaf(mut bits) = node.data {
            let index = y * 8 + x;
            if alive {
                bits |= 1 << index;
            } else {
                bits &= !(1 << index);
            }
            return self.cache.get_node(NodeData::Leaf(bits));
        }

        if let NodeData::Branch { nw, ne, sw, se, .. } = &node.data {
            let half_size = size / 2;
            let (mut new_nw, mut new_ne, mut new_sw, mut new_se) =
                (nw.clone(), ne.clone(), sw.clone(), se.clone());

            if x < half_size {
                if y < half_size {
                    new_nw = self.recursive_set(nw.clone(), half_size, x, y, alive);
                } else {
                    new_sw = self.recursive_set(sw.clone(), half_size, x, y - half_size, alive);
                }
            } else {
                if y < half_size {
                    new_ne = self.recursive_set(ne.clone(), half_size, x - half_size, y, alive);
                } else {
                    new_se = self.recursive_set(
                        se.clone(),
                        half_size,
                        x - half_size,
                        y - half_size,
                        alive,
                    );
                }
            }
            return self.cache.join(new_nw, new_ne, new_sw, new_se);
        }
        unreachable!();
    }

    fn recursive_get(&self, node: Arc<Node>, size: u64, x: u64, y: u64) -> bool {
        if node.population == 0 {
            return false;
        }

        match &node.data {
            NodeData::Leaf(bits) => {
                let index = y * 8 + x;
                (bits >> index) & 1 == 1
            }
            NodeData::Branch { nw, ne, sw, se, .. } => {
                let half_size = size / 2;
                if x < half_size {
                    if y < half_size {
                        self.recursive_get(nw.clone(), half_size, x, y)
                    } else {
                        self.recursive_get(sw.clone(), half_size, x, y - half_size)
                    }
                } else {
                    if y < half_size {
                        self.recursive_get(ne.clone(), half_size, x - half_size, y)
                    } else {
                        self.recursive_get(se.clone(), half_size, x - half_size, y - half_size)
                    }
                }
            }
        }
    }

    fn recursive_export(
        &self,
        node: &Arc<Node>,
        x: i64,
        y: i64,
        size: u64,
        list: &mut Vec<(i64, i64)>,
    ) {
        if node.population == 0 {
            return;
        }

        match &node.data {
            NodeData::Leaf(bits) => {
                for row in 0..8 {
                    for col in 0..8 {
                        if (bits >> (row * 8 + col)) & 1 == 1 {
                            list.push((x + col as i64, y + row as i64));
                        }
                    }
                }
            }
            NodeData::Branch { nw, ne, sw, se, .. } => {
                let half = (size / 2) as i64;
                // Branch coordinate offsets:
                // NW is (0,0) relative to node origin
                self.recursive_export(nw, x, y, size / 2, list);
                // NE is (half, 0)
                self.recursive_export(ne, x + half, y, size / 2, list);
                // SW is (0, half)
                self.recursive_export(sw, x, y + half, size / 2, list);
                // SE is (half, half)
                self.recursive_export(se, x + half, y + half, size / 2, list);
            }
        }
    }

    fn recursive_draw(
        &self,
        node: &Arc<Node>,
        x: f64,
        y: f64,
        size: f64,
        buffer: &mut [u8],
        width: usize,
        height: usize,
        buffer_w_px: f64,
        buffer_h_px: f64,
    ) {
        // 1. Optimization: Empty nodes contribute nothing
        if node.population == 0 {
            return;
        }

        // 2. Bounds Check
        if x >= buffer_w_px || y >= buffer_h_px || x + size <= 0.0 || y + size <= 0.0 {
            return;
        }

        // 3. Pixel / Sub-pixel Drawing
        if size <= 1.0 {
            self.fill_rect(buffer, width, height, x, y, size);
            return;
        }

        match &node.data {
            NodeData::Leaf(bits) => {
                let cell_size = size / 8.0;

                for row in 0..8 {
                    for col in 0..8 {
                        if (bits >> (row * 8 + col)) & 1 == 1 {
                            let cx = x + (col as f64 * cell_size);
                            let cy = y + (row as f64 * cell_size);

                            self.fill_rect(buffer, width, height, cx, cy, cell_size);
                        }
                    }
                }
            }
            NodeData::Branch { nw, ne, sw, se, .. } => {
                let half = size / 2.0;
                self.recursive_draw(
                    nw,
                    x,
                    y,
                    half,
                    buffer,
                    width,
                    height,
                    buffer_w_px,
                    buffer_h_px,
                );
                self.recursive_draw(
                    ne,
                    x + half,
                    y,
                    half,
                    buffer,
                    width,
                    height,
                    buffer_w_px,
                    buffer_h_px,
                );
                self.recursive_draw(
                    sw,
                    x,
                    y + half,
                    half,
                    buffer,
                    width,
                    height,
                    buffer_w_px,
                    buffer_h_px,
                );
                self.recursive_draw(
                    se,
                    x + half,
                    y + half,
                    half,
                    buffer,
                    width,
                    height,
                    buffer_w_px,
                    buffer_h_px,
                );
            }
        }
    }

    fn fill_rect(&self, buffer: &mut [u8], width: usize, height: usize, x: f64, y: f64, size: f64) {
        let max_w_px = width as f64;
        let start_x = x.floor().max(0.0) as usize;
        let start_y = y.floor().max(0.0) as usize;
        let end_x = (x + size).ceil().min(max_w_px) as usize;
        let end_y = (y + size).ceil().min(height as f64) as usize;

        if start_x >= end_x || start_y >= end_y {
            return;
        }

        for py in start_y..end_y {
            let row_offset = py * width;
            for px in start_x..end_x {
                buffer[row_offset + px] = 1;
            }
        }
    }
}

impl LifeEngine for Hashlife {
    fn name(&self) -> &str {
        "Hashlife"
    }

    fn population(&self) -> u64 {
        self.root.population
    }

    fn set_cell(&mut self, x: i64, y: i64, alive: bool) {
        self.expand_to_fit(x, y);

        let size = 1u64 << self.root.level();
        let rel_x = (x - self.origin_x) as u64;
        let rel_y = (y - self.origin_y) as u64;

        let new_root = self.recursive_set(self.root.clone(), size, rel_x, rel_y, alive);
        self.root = new_root;
    }

    fn get_cell(&self, x: i64, y: i64) -> bool {
        let size = 1u64 << self.root.level();
        let rel_x = x - self.origin_x;
        let rel_y = y - self.origin_y;

        // If coordinate is outside current universe bounds, it's definitely dead
        if rel_x < 0 || rel_y < 0 || rel_x >= size as i64 || rel_y >= size as i64 {
            return false;
        }

        self.recursive_get(self.root.clone(), size as u64, rel_x as u64, rel_y as u64)
    }

    fn clear(&mut self) {
        self.root = self.cache.empty_node(4);
        self.origin_x = 0;
        self.origin_y = 0;
        self.generation = 0;
    }

    fn export(&self) -> Vec<(i64, i64)> {
        let mut alive_cells = Vec::new();
        let size = 1u64 << self.root.level();
        self.recursive_export(
            &self.root,
            self.origin_x,
            self.origin_y,
            size,
            &mut alive_cells,
        );
        alive_cells
    }

    fn import(&mut self, alive_cells: Vec<(i64, i64)>) {
        self.clear();
        for (x, y) in alive_cells {
            self.set_cell(x, y, true);
        }
    }

    fn step(&mut self, _steps: u64) -> u64 {
        // 1. Expansion Phase
        // Aggressively expand if the pattern is growing.
        // We need padding to ensure the result (which is half the size of root)
        // still covers the active area after the time step.
        for _ in 0..60 {
            let too_small = self.root.level() < 5;
            let needs_padding = !self.is_padded();

            if too_small || needs_padding {
                self.expand();
            } else {
                break;
            }
        }

        // 2. Evolution Phase
        // evolve() returns the center of the universe advanced by 2^(level-2) generations.
        // It returns a node 1 level smaller.
        let next_node = self.cache.evolve(self.root.clone());
        self.root = next_node;

        // 3. Origin Update Phase
        // The new root is the spatial center of the old root.
        // The center is offset by (old_size / 4) in both X and Y.
        // old_size / 4 == 2^(old_level) / 2^2 == 2^(old_level - 2).
        // Since root.level() is now (old_level - 1), this is 2^(current_level - 1).
        let shift = 1i64 << (self.root.level() - 1);
        self.origin_x += shift;
        self.origin_y += shift;

        // 4. Calculate Steps Done
        let steps_done = 1u64 << (self.root.level() - 2);
        self.generation += steps_done;

        steps_done
    }

    fn draw_to_buffer(&self, rect: Rect, buffer: &mut [u8], width: usize, height: usize) {
        buffer.fill(0);

        let buffer_w_px = width as f64;
        if rect.width() <= 0.0 {
            return;
        }

        let scale = buffer_w_px / rect.width() as f64;
        let root_screen_x = (self.origin_x as f64 - rect.min.x as f64) * scale;
        let root_screen_y = (self.origin_y as f64 - rect.min.y as f64) * scale;
        let root_size_world = (1u64 << self.root.level()) as f64;
        let root_size_px = root_size_world * scale;
        let buffer_h_px = height as f64;

        self.recursive_draw(
            &self.root,
            root_screen_x,
            root_screen_y,
            root_size_px,
            buffer,
            width,
            height,
            buffer_w_px,
            buffer_h_px,
        );
    }
}
