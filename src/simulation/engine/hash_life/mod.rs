mod cache;
mod node;

use crate::simulation::engine::LifeEngine;
use bevy::math::{I64Vec2, Rect};
use cache::HashLifeCache;
use node::{Node, NodeData};
use std::sync::Arc;

#[derive(Clone)]
pub struct HashLife {
    cache: HashLifeCache,
    root: Arc<Node>,
    generation: u64,
    origin_x: i64,
    origin_y: i64,
}

impl HashLife {
    /// Initializes a new Hashlife universe with a Level 4 (16x16) empty grid.
    pub fn new() -> Self {
        let mut cache = HashLifeCache::new();
        let root = cache.empty_node(4);

        HashLife {
            cache,
            root,
            generation: 0,
            origin_x: 0,
            origin_y: 0,
        }
    }
}

impl LifeEngine for HashLife {
    fn id(&self) -> &str {
        "hash-life"
    }

    fn name(&self) -> &str {
        "HashLife"
    }

    fn population(&self) -> u64 {
        self.root.population
    }

    fn set_cell(&mut self, pos: I64Vec2, alive: bool) {
        self.set_cells(&[pos], alive);
    }

    fn set_cells(&mut self, coords: &[I64Vec2], alive: bool) {
        let points: Vec<(i64, i64)> = coords.iter().map(|p| (p.x, p.y)).collect();
        self.apply_batch(points, alive);
    }

    fn get_cell(&self, pos: I64Vec2) -> bool {
        let size = 1u64 << self.root.level();
        let rel_x = pos.x - self.origin_x;
        let rel_y = pos.y - self.origin_y;

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

    fn export(&self) -> Vec<I64Vec2> {
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
            .into_iter()
            .map(|(x, y)| I64Vec2::new(x, y))
            .collect()
    }

    fn import(&mut self, alive_cells: &[I64Vec2]) {
        self.clear();
        self.set_cells(alive_cells, true);
    }

    /// Advances the simulation by `steps` generations.
    ///
    /// Hashlife naturally steps forward by $2^{k-2}$ generations where $k$ is the level.
    /// To support arbitrary step counts, we use binary decomposition: taking the
    /// largest possible power-of-two jump that doesn't exceed the remaining steps.
    fn step(&mut self, mut steps: u64) -> u64 {
        if steps == 0 {
            return 0;
        }

        let total_steps = steps;

        while steps > 0 {
            // 1. Ensure universe is padded with enough empty space
            for _ in 0..60 {
                let too_small = self.root.level() < 5;
                if too_small || !self.is_padded() {
                    self.expand();
                } else {
                    break;
                }
            }

            // 2. Determine max jump size (2^(level-2))
            let max_step_power = self.root.level() - 2;
            let max_jump = 1u64 << max_step_power;

            // 3. Evolve
            let (next_node, steps_taken) = if steps >= max_jump {
                (self.cache.evolve(self.root.clone()), max_jump)
            } else {
                (self.cache.evolve_1(self.root.clone()), 1)
            };

            self.root = next_node;
            steps -= steps_taken;

            // 4. Update Origin
            // The result of evolve() is spatially located in the center of the previous node,
            // effectively shifting the origin by half a quadrant size.
            let shift = 1i64 << (self.root.level() - 1);
            self.origin_x += shift;
            self.origin_y += shift;
        }

        self.generation += total_steps;
        total_steps
    }

    fn draw_to_buffer(&self, rect: Rect, buffer: &mut [u8], width: usize, height: usize) {
        buffer.fill(0);
        if rect.width() <= 0.0 {
            return;
        }

        let buffer_w = width as f64;
        let buffer_h = height as f64;

        let scale = buffer_w / rect.width() as f64;
        let root_screen_x = (self.origin_x as f64 - rect.min.x as f64) * scale;
        let root_screen_y = (self.origin_y as f64 - rect.min.y as f64) * scale;
        let root_size_world = (1u64 << self.root.level()) as f64;
        let root_size_px = root_size_world * scale;

        self.recursive_draw(
            &self.root,
            root_screen_x,
            root_screen_y,
            root_size_px,
            buffer,
            width,
            height,
            buffer_w,
            buffer_h,
        );
    }

    fn box_clone(&self) -> Box<dyn LifeEngine> {
        Box::new(self.clone())
    }
}

impl HashLife {
    /// Checks if the active population is contained within the inner 50% of the node.
    /// This is required before evolution to ensure patterns don't grow outside the bounds.
    fn is_padded(&self) -> bool {
        let NodeData::Branch { nw, ne, sw, se, .. } = &self.root.data else {
            return false;
        };

        // Helper to get population of specific quadrants from children
        let get_sub = |node: &Arc<Node>, target_quad: usize| -> u64 {
            match &node.data {
                NodeData::Branch { nw, ne, sw, se, .. } => match target_quad {
                    0 => nw.population, // NW
                    1 => ne.population, // NE
                    2 => sw.population, // SW
                    3 => se.population, // SE
                    _ => 0,
                },
                _ => 0,
            }
        };

        // We sum the populations of the "inner" quadrants of the children:
        // NW's SE + NE's SW + SW's NE + SE's NW
        let inner_pop = get_sub(nw, 3) + get_sub(ne, 2) + get_sub(sw, 1) + get_sub(se, 0);

        self.root.population == inner_pop
    }

    /// Expands the universe until the given coordinate fits within the bounds.
    fn expand_to_fit(&mut self, x: i64, y: i64) {
        // Safety cap to prevent infinite loops on extreme coordinates
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

    /// Wraps the current root node in a larger empty context.
    /// This doubles the size of the universe and centers the old root.
    fn expand(&mut self) {
        let root = self.root.clone();

        match &root.data {
            NodeData::Branch {
                nw,
                ne,
                sw,
                se,
                level,
            } => {
                let empty = self.cache.empty_node(level - 1);

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

                let shift = 1i64 << (level - 1);
                self.origin_x -= shift;
                self.origin_y -= shift;
            }
            NodeData::Leaf(_) => {
                let empty = self.cache.empty_node(root.level());
                self.root =
                    self.cache
                        .join(root.clone(), empty.clone(), empty.clone(), empty.clone());
            }
        }
    }

    /// Batched updates for efficient tree traversal.
    /// Sorts points to maximize cache locality and minimize tree re-traversal.
    fn apply_batch(&mut self, mut points: Vec<(i64, i64)>, alive: bool) {
        if points.is_empty() {
            return;
        }

        // 1. Expand universe
        for &(x, y) in &points {
            self.expand_to_fit(x, y);
        }

        // 2. Sort points (Y then X, or Morton order)
        points.sort_unstable_by(|a, b| {
            if a.1 != b.1 {
                a.1.cmp(&b.1)
            } else {
                a.0.cmp(&b.0)
            }
        });

        // 3. Convert to relative coordinates
        let rel_points: Vec<(u64, u64)> = points
            .iter()
            .map(|(x, y)| ((x - self.origin_x) as u64, (y - self.origin_y) as u64))
            .collect();

        // 4. Recursive Set
        let size = 1u64 << self.root.level();
        self.root = self.recursive_set_batch(self.root.clone(), size, 0, 0, &rel_points, alive);
    }

    fn recursive_set_batch(
        &mut self,
        node: Arc<Node>,
        size: u64,
        offset_x: u64,
        offset_y: u64,
        sorted_points: &[(u64, u64)],
        alive: bool,
    ) -> Arc<Node> {
        if sorted_points.is_empty() {
            return node;
        }

        if let NodeData::Leaf(mut bits) = node.data {
            for &(px, py) in sorted_points {
                let lx = px - offset_x;
                let ly = py - offset_y;

                if lx < 8 && ly < 8 {
                    let index = ly * 8 + lx;
                    if alive {
                        bits |= 1 << index;
                    } else {
                        bits &= !(1 << index);
                    }
                }
            }
            return self.cache.get_node(NodeData::Leaf(bits));
        }

        if let NodeData::Branch { nw, ne, sw, se, .. } = &node.data {
            let half = size / 2;

            // Partition points into quadrants
            // Since points are sorted, we can do this efficiently without full scans,
            // but for simplicity/robustness we filter.
            let mut pts_nw = Vec::new();
            let mut pts_ne = Vec::new();
            let mut pts_sw = Vec::new();
            let mut pts_se = Vec::new();

            for &(px, py) in sorted_points {
                let lx = px - offset_x;
                let ly = py - offset_y;

                if lx < half {
                    if ly < half {
                        pts_nw.push((px, py));
                    } else {
                        pts_sw.push((px, py));
                    }
                } else {
                    if ly < half {
                        pts_ne.push((px, py));
                    } else {
                        pts_se.push((px, py));
                    }
                }
            }

            let new_nw =
                self.recursive_set_batch(nw.clone(), half, offset_x, offset_y, &pts_nw, alive);
            let new_ne = self.recursive_set_batch(
                ne.clone(),
                half,
                offset_x + half,
                offset_y,
                &pts_ne,
                alive,
            );
            let new_sw = self.recursive_set_batch(
                sw.clone(),
                half,
                offset_x,
                offset_y + half,
                &pts_sw,
                alive,
            );
            let new_se = self.recursive_set_batch(
                se.clone(),
                half,
                offset_x + half,
                offset_y + half,
                &pts_se,
                alive,
            );

            return self.cache.join(new_nw, new_ne, new_sw, new_se);
        }

        unreachable!()
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
                let half = size / 2;
                if x < half {
                    if y < half {
                        self.recursive_get(nw.clone(), half, x, y)
                    } else {
                        self.recursive_get(sw.clone(), half, x, y - half)
                    }
                } else {
                    if y < half {
                        self.recursive_get(ne.clone(), half, x - half, y)
                    } else {
                        self.recursive_get(se.clone(), half, x - half, y - half)
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
                self.recursive_export(nw, x, y, size / 2, list);
                self.recursive_export(ne, x + half, y, size / 2, list);
                self.recursive_export(sw, x, y + half, size / 2, list);
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
        max_w: f64,
        max_h: f64,
    ) {
        if node.population == 0 {
            return;
        }

        // Culling: if completely off-screen
        if x >= max_w || y >= max_h || x + size <= 0.0 || y + size <= 0.0 {
            return;
        }

        // LOD: if a node is smaller than a pixel, draw it as a solid block
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
                self.recursive_draw(nw, x, y, half, buffer, width, height, max_w, max_h);
                self.recursive_draw(ne, x + half, y, half, buffer, width, height, max_w, max_h);
                self.recursive_draw(sw, x, y + half, half, buffer, width, height, max_w, max_h);
                self.recursive_draw(
                    se,
                    x + half,
                    y + half,
                    half,
                    buffer,
                    width,
                    height,
                    max_w,
                    max_h,
                );
            }
        }
    }

    fn fill_rect(&self, buffer: &mut [u8], width: usize, height: usize, x: f64, y: f64, size: f64) {
        let start_x = x.round().max(0.0) as usize;
        let start_y = y.round().max(0.0) as usize;

        let end_x = (x + size).round().min(width as f64) as usize;
        let end_y = (y + size).round().min(height as f64) as usize;

        if start_x >= end_x || start_y >= end_y {
            return;
        }

        for py in start_y..end_y {
            let row_offset = py * width;
            let row_slice = &mut buffer[row_offset + start_x..row_offset + end_x];
            row_slice.fill(255);
        }
    }
}
