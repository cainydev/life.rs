use super::node::{Node, NodeData};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, OnceLock};

pub struct HashlifeCache {
    map: HashMap<NodeData, Arc<Node>>,
    pub empty_nodes: Vec<Arc<Node>>,
}

impl HashlifeCache {
    pub fn new() -> Self {
        let base_data = NodeData::Leaf(0);

        // Calculate hash for the base empty node
        let mut hasher = DefaultHasher::default();
        base_data.hash(&mut hasher);
        let base_hash = hasher.finish();

        let base_empty = Arc::new(Node {
            data: base_data.clone(),
            population: 0,
            hash: base_hash,
            result: OnceLock::new(),
        });

        let mut map = HashMap::new();
        map.insert(base_data, base_empty.clone());

        Self {
            map,
            empty_nodes: vec![base_empty],
        }
    }

    pub fn get_node(&mut self, data: NodeData) -> Arc<Node> {
        if let Some(node) = self.map.get(&data) {
            return node.clone();
        }

        let population = match &data {
            NodeData::Leaf(bits) => bits.count_ones() as u64,
            NodeData::Branch { nw, ne, sw, se, .. } => {
                nw.population + ne.population + sw.population + se.population
            }
        };

        let mut hasher = DefaultHasher::default();
        data.hash(&mut hasher);
        let hash = hasher.finish();

        let node = Arc::new(Node {
            data: data.clone(),
            population,
            hash,
            result: OnceLock::new(),
        });

        self.map.insert(data, node.clone());
        node
    }

    pub fn join(
        &mut self,
        nw: Arc<Node>,
        ne: Arc<Node>,
        sw: Arc<Node>,
        se: Arc<Node>,
    ) -> Arc<Node> {
        let level = nw.level() + 1;
        // Safety check (optional but good for debugging)
        debug_assert_eq!(nw.level(), ne.level());
        debug_assert_eq!(nw.level(), sw.level());
        debug_assert_eq!(nw.level(), se.level());

        self.get_node(NodeData::Branch {
            nw,
            ne,
            sw,
            se,
            level,
        })
    }

    // Returns a node half the size, stepped forward in time.
    pub fn evolve(&mut self, node: Arc<Node>) -> Arc<Node> {
        // 1. Check Cache (Memoization)
        if let Some(res) = node.result.get() {
            return res.clone();
        }

        // 2. Calculate Result (if not in cache)
        let result = match &node.data {
            NodeData::Leaf(bits) => self.calc_leaf(*bits),
            NodeData::Branch {
                nw,
                ne,
                sw,
                se,
                level,
            } => self.calc_branch(nw, ne, sw, se, *level),
        };

        // 3. Save to Cache
        let _ = node.result.set(result.clone());
        result
    }

    fn calc_leaf(&mut self, input: u64) -> Arc<Node> {
        let mut output = 0u64;
        for y in 0..8 {
            for x in 0..8 {
                let mut neighbors = 0;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x + dx;
                        let ny = y + dy;
                        if nx >= 0 && nx < 8 && ny >= 0 && ny < 8 {
                            if (input >> (ny * 8 + nx)) & 1 == 1 {
                                neighbors += 1;
                            }
                        }
                    }
                }
                let is_alive = (input >> (y * 8 + x)) & 1 == 1;
                if neighbors == 3 || (is_alive && neighbors == 2) {
                    output |= 1 << (y * 8 + x);
                }
            }
        }
        self.get_node(NodeData::Leaf(output))
    }

    fn calc_level_4_base(
        &mut self,
        nw: &Arc<Node>,
        ne: &Arc<Node>,
        sw: &Arc<Node>,
        se: &Arc<Node>,
    ) -> Arc<Node> {
        let (
            NodeData::Leaf(nw_bits),
            NodeData::Leaf(ne_bits),
            NodeData::Leaf(sw_bits),
            NodeData::Leaf(se_bits),
        ) = (&nw.data, &ne.data, &sw.data, &se.data)
        else {
            panic!("Level 4 node children must be Leaves");
        };

        // Construct 16x16 grid
        // NW is (0..8, 0..8), NE is (8..16, 0..8) [x, y]
        // Bit logic is Row Major: (y * 8 + x)
        let mut grid = [[false; 16]; 16];

        for y in 0..8 {
            for x in 0..8 {
                grid[y][x] = (nw_bits >> (y * 8 + x)) & 1 == 1;
                grid[y][x + 8] = (ne_bits >> (y * 8 + x)) & 1 == 1;
                grid[y + 8][x] = (sw_bits >> (y * 8 + x)) & 1 == 1;
                grid[y + 8][x + 8] = (se_bits >> (y * 8 + x)) & 1 == 1;
            }
        }

        // Run simulation for 4 generations
        for _ in 0..4 {
            let mut next_grid = [[false; 16]; 16];
            for y in 0..16 {
                for x in 0..16 {
                    let mut neighbors = 0;
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let ny = y as isize + dy;
                            let nx = x as isize + dx;

                            // Void boundary condition (cells outside 16x16 are dead)
                            if ny >= 0 && ny < 16 && nx >= 0 && nx < 16 {
                                if grid[ny as usize][nx as usize] {
                                    neighbors += 1;
                                }
                            }
                        }
                    }
                    let alive = grid[y][x];
                    next_grid[y][x] = neighbors == 3 || (alive && neighbors == 2);
                }
            }
            grid = next_grid;
        }

        // Extract center 8x8 (from index 4 to 11 inclusive)
        let mut result_bits = 0u64;
        for y in 0..8 {
            for x in 0..8 {
                if grid[y + 4][x + 4] {
                    result_bits |= 1 << (y * 8 + x);
                }
            }
        }

        self.get_node(NodeData::Leaf(result_bits))
    }

    fn calc_branch(
        &mut self,
        nw: &Arc<Node>,
        ne: &Arc<Node>,
        sw: &Arc<Node>,
        se: &Arc<Node>,
        level: u8,
    ) -> Arc<Node> {
        // FIX: Base case for recursion.
        // If we are at Level 4, children are Leaves. We cannot recurse
        // using standard logic because evolve(Leaf) returns Leaf (Level 3),
        // which would lead to infinite recursion.
        if level == 4 {
            return self.calc_level_4_base(nw, ne, sw, se);
        }

        // --- Existing Logic for Level > 4 ---

        // 1. Construct the 9 overlapping sub-squares (Level K-1)
        let n00 = nw.clone();
        let n01 = self.centered_horizontal(nw, ne);
        let n02 = ne.clone();

        let n10 = self.centered_vertical(nw, sw);
        let n11 = self.centered_sub(nw, ne, sw, se);
        let n12 = self.centered_vertical(ne, se);

        let n20 = sw.clone();
        let n21 = self.centered_horizontal(sw, se);
        let n22 = se.clone();

        // 2. Evolve the 9 squares
        let r00 = self.evolve(n00);
        let r01 = self.evolve(n01);
        let r02 = self.evolve(n02);
        let r10 = self.evolve(n10);
        let r11 = self.evolve(n11);
        let r12 = self.evolve(n12);
        let r20 = self.evolve(n20);
        let r21 = self.evolve(n21);
        let r22 = self.evolve(n22);

        // 3. Combine results into 4 overlapping squares
        let q_nw = self.join(r00.clone(), r01.clone(), r10.clone(), r11.clone());
        let q_ne = self.join(r01.clone(), r02.clone(), r11.clone(), r12.clone());
        let q_sw = self.join(r10.clone(), r11.clone(), r20.clone(), r21.clone());
        let q_se = self.join(r11, r12, r21, r22);

        // 4. Evolve the 4 squares
        let final_nw = self.evolve(q_nw);
        let final_ne = self.evolve(q_ne);
        let final_sw = self.evolve(q_sw);
        let final_se = self.evolve(q_se);

        // 5. Compose the final result
        self.join(final_nw, final_ne, final_sw, final_se)
    }

    fn centered_sub(
        &mut self,
        nw: &Arc<Node>,
        ne: &Arc<Node>,
        sw: &Arc<Node>,
        se: &Arc<Node>,
    ) -> Arc<Node> {
        match (&nw.data, &ne.data, &sw.data, &se.data) {
            // Case 1: Bits (Level 4 -> Children are Level 3 Leaves)
            (
                NodeData::Leaf(nw_bits),
                NodeData::Leaf(ne_bits),
                NodeData::Leaf(sw_bits),
                NodeData::Leaf(se_bits),
            ) => self.centered_bits(*nw_bits, *ne_bits, *sw_bits, *se_bits),

            // Case 2: Branches (Level > 4)
            (
                NodeData::Branch { se: nw_se, .. },
                NodeData::Branch { sw: ne_sw, .. },
                NodeData::Branch { ne: sw_ne, .. },
                NodeData::Branch { nw: se_nw, .. },
            ) => self.join(nw_se.clone(), ne_sw.clone(), sw_ne.clone(), se_nw.clone()),

            _ => panic!("Mismatched node levels in centered_sub"),
        }
    }

    fn centered_horizontal(&mut self, left: &Arc<Node>, right: &Arc<Node>) -> Arc<Node> {
        match (&left.data, &right.data) {
            (NodeData::Leaf(l_bits), NodeData::Leaf(r_bits)) => {
                // FIX: Manually construct the shifted bitmask instead of using centered_bits
                // We want Left's East half (Cols 4-7) and Right's West half (Cols 0-3)
                let mut res = 0u64;
                for y in 0..8 {
                    // Left Node Cols 4..8 -> Result Cols 0..4
                    for x in 4..8 {
                        if (l_bits >> (y * 8 + x)) & 1 == 1 {
                            res |= 1 << (y * 8 + (x - 4));
                        }
                    }
                    // Right Node Cols 0..4 -> Result Cols 4..8
                    for x in 0..4 {
                        if (r_bits >> (y * 8 + x)) & 1 == 1 {
                            res |= 1 << (y * 8 + (x + 4));
                        }
                    }
                }
                self.get_node(NodeData::Leaf(res))
            }
            (
                NodeData::Branch {
                    ne: l_ne, se: l_se, ..
                },
                NodeData::Branch {
                    nw: r_nw, sw: r_sw, ..
                },
            ) => self.join(l_ne.clone(), r_nw.clone(), l_se.clone(), r_sw.clone()),
            _ => panic!("Mismatched levels"),
        }
    }

    fn centered_vertical(&mut self, top: &Arc<Node>, bottom: &Arc<Node>) -> Arc<Node> {
        match (&top.data, &bottom.data) {
            (NodeData::Leaf(t_bits), NodeData::Leaf(b_bits)) => {
                let mut res = 0u64;
                // Top Node Rows 4..8 -> Result Rows 0..4
                for y in 4..8 {
                    for x in 0..8 {
                        if (t_bits >> (y * 8 + x)) & 1 == 1 {
                            res |= 1 << ((y - 4) * 8 + x);
                        }
                    }
                }
                // Bottom Node Rows 0..4 -> Result Rows 4..8
                for y in 0..4 {
                    for x in 0..8 {
                        if (b_bits >> (y * 8 + x)) & 1 == 1 {
                            res |= 1 << ((y + 4) * 8 + x);
                        }
                    }
                }
                self.get_node(NodeData::Leaf(res))
            }
            (
                NodeData::Branch {
                    sw: t_sw, se: t_se, ..
                },
                NodeData::Branch {
                    nw: b_nw, ne: b_ne, ..
                },
            ) => self.join(t_sw.clone(), t_se.clone(), b_nw.clone(), b_ne.clone()),
            _ => panic!("Mismatched levels"),
        }
    }

    // Takes 4 8x8 grids, extracts the inner corners, and forms a new 8x8 grid.
    fn centered_bits(&mut self, nw: u64, ne: u64, sw: u64, se: u64) -> Arc<Node> {
        let mut res = 0u64;

        for y in 0..8 {
            for x in 0..8 {
                let global_x = x + 4;
                let global_y = y + 4;

                let bit = if global_x < 8 && global_y < 8 {
                    (nw >> (global_y * 8 + global_x)) & 1
                } else if global_x >= 8 && global_y < 8 {
                    (ne >> (global_y * 8 + (global_x - 8))) & 1
                } else if global_x < 8 && global_y >= 8 {
                    (sw >> ((global_y - 8) * 8 + global_x)) & 1
                } else {
                    (se >> ((global_y - 8) * 8 + (global_x - 8))) & 1
                };

                if bit == 1 {
                    res |= 1 << (y * 8 + x);
                }
            }
        }

        self.get_node(NodeData::Leaf(res))
    }

    // Returns an empty node at the specified level
    pub fn empty_node(&mut self, level: u8) -> Arc<Node> {
        if level <= 3 {
            return self.empty_nodes[0].clone();
        }

        let index = (level - 3) as usize;
        if index < self.empty_nodes.len() {
            return self.empty_nodes[index].clone();
        }

        let child = self.empty_node(level - 1);
        let node = self.join(child.clone(), child.clone(), child.clone(), child.clone());

        self.empty_nodes.push(node.clone());
        node
    }
}
