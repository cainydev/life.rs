use super::node::{Node, NodeData};
use rustc_hash::{FxHashMap, FxHasher};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock};

#[derive(Clone)]
pub struct HashLifeCache {
    map: FxHashMap<NodeData, Arc<Node>>,
    pub empty_nodes: Vec<Arc<Node>>,
}

impl HashLifeCache {
    /// Creates a new cache initialized with the base empty leaf node.
    pub fn new() -> Self {
        let base_data = NodeData::Leaf(0);

        let mut hasher = FxHasher::default();
        base_data.hash(&mut hasher);
        let base_hash = hasher.finish();

        let base_empty = Arc::new(Node {
            data: base_data.clone(),
            population: 0,
            hash: base_hash,
            result: OnceLock::new(),
            result_step_1: OnceLock::new(),
        });

        let mut map = FxHashMap::default();
        map.insert(base_data, base_empty.clone());

        Self {
            map,
            empty_nodes: vec![base_empty],
        }
    }

    /// Advances the node by $2^{level-2}$ generations.
    pub fn evolve(&mut self, node: Arc<Node>) -> Arc<Node> {
        if let Some(res) = node.result.get() {
            return res.clone();
        }

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

        let _ = node.result.set(result.clone());
        result
    }

    /// Advances the node by exactly 1 generation.
    pub fn evolve_1(&mut self, node: Arc<Node>) -> Arc<Node> {
        if let Some(res) = node.result_step_1.get() {
            return res.clone();
        }

        let result = match &node.data {
            // Level 3 (Leaf): Standard calc_leaf does 1 step logic
            NodeData::Leaf(bits) => self.calc_leaf(*bits),

            // Level 4 (16x16): Optimized 64-bit grid simulation
            NodeData::Branch {
                nw,
                ne,
                sw,
                se,
                level,
            } if *level == 4 => self.calc_level_4_grid(nw, ne, sw, se, 1),

            // Level > 4: Recursive decomposition
            NodeData::Branch { nw, ne, sw, se, .. } => {
                let n00 = nw.clone();
                let n01 = self.centered_horizontal(nw, ne);
                let n02 = ne.clone();
                let n10 = self.centered_vertical(nw, sw);
                let n11 = self.centered_sub(nw, ne, sw, se);
                let n12 = self.centered_vertical(ne, se);
                let n20 = sw.clone();
                let n21 = self.centered_horizontal(sw, se);
                let n22 = se.clone();

                let r00 = self.evolve_1(n00);
                let r01 = self.evolve_1(n01);
                let r02 = self.evolve_1(n02);
                let r10 = self.evolve_1(n10);
                let r11 = self.evolve_1(n11);
                let r12 = self.evolve_1(n12);
                let r20 = self.evolve_1(n20);
                let r21 = self.evolve_1(n21);
                let r22 = self.evolve_1(n22);

                let c_nw = self.centered_sub(&r00, &r01, &r10, &r11);
                let c_ne = self.centered_sub(&r01, &r02, &r11, &r12);
                let c_sw = self.centered_sub(&r10, &r11, &r20, &r21);
                let c_se = self.centered_sub(&r11, &r12, &r21, &r22);

                self.join(c_nw, c_ne, c_sw, c_se)
            }
        };

        let _ = node.result_step_1.set(result.clone());
        result
    }

    /// Returns a canonical empty node for the given level, creating it if necessary.
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

    #[allow(unused)]
    /// Removes unreferenced nodes from the internal map.
    pub fn collect_garbage(&mut self) -> usize {
        let before = self.map.len();
        self.map.retain(|_, node| Arc::strong_count(node) > 1);
        before - self.map.len()
    }

    /// Canonicalizes a node: returns an existing node from the cache or creates a new one.
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

        let mut hasher = FxHasher::default();
        data.hash(&mut hasher);
        let hash = hasher.finish();

        let node = Arc::new(Node {
            data: data.clone(),
            population,
            hash,
            result: OnceLock::new(),
            result_step_1: OnceLock::new(),
        });

        self.map.insert(data, node.clone());
        node
    }

    /// Combines four children into a new branch node one level higher.
    pub fn join(
        &mut self,
        nw: Arc<Node>,
        ne: Arc<Node>,
        sw: Arc<Node>,
        se: Arc<Node>,
    ) -> Arc<Node> {
        let level = nw.level() + 1;
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

    /// Calculates the next state for a Leaf node (8x8 grid).
    /// Uses SWAR (SIMD Within A Register) techniques for parallel counting.
    fn calc_leaf(&mut self, input: u64) -> Arc<Node> {
        if input == 0 {
            return self.empty_nodes[0].clone();
        }

        let l = (input >> 1) & 0x7F7F7F7F7F7F7F7F;
        let r = (input << 1) & 0xFEFEFEFEFEFEFEFE;
        let u = input << 8;
        let d = input >> 8;
        let ul = (u >> 1) & 0x7F7F7F7F7F7F7F7F;
        let ur = (u << 1) & 0xFEFEFEFEFEFEFEFE;
        let dl = (d >> 1) & 0x7F7F7F7F7F7F7F7F;
        let dr = (d << 1) & 0xFEFEFEFEFEFEFEFE;

        // Parallel Neighbor Counting (Adder Tree)
        // Sum 8 inputs into 3 bits: a (1s), b (2s), c (4s).
        // Logic: a + b*2 + c*4 = number of neighbors
        let mut a = 0;
        let mut b = 0;
        let mut c = 0;

        let neighbors = [l, r, u, d, ul, ur, dl, dr];

        // Manual unroll for efficiency
        let n = neighbors[0];
        let c_ab = a & n;
        a ^= n;
        let c_bc = b & c_ab;
        b ^= c_ab;
        c |= c_bc;
        let n = neighbors[1];
        let c_ab = a & n;
        a ^= n;
        let c_bc = b & c_ab;
        b ^= c_ab;
        c |= c_bc;
        let n = neighbors[2];
        let c_ab = a & n;
        a ^= n;
        let c_bc = b & c_ab;
        b ^= c_ab;
        c |= c_bc;
        let n = neighbors[3];
        let c_ab = a & n;
        a ^= n;
        let c_bc = b & c_ab;
        b ^= c_ab;
        c |= c_bc;
        let n = neighbors[4];
        let c_ab = a & n;
        a ^= n;
        let c_bc = b & c_ab;
        b ^= c_ab;
        c |= c_bc;
        let n = neighbors[5];
        let c_ab = a & n;
        a ^= n;
        let c_bc = b & c_ab;
        b ^= c_ab;
        c |= c_bc;
        let n = neighbors[6];
        let c_ab = a & n;
        a ^= n;
        let c_bc = b & c_ab;
        b ^= c_ab;
        c |= c_bc;
        let n = neighbors[7];
        let c_ab = a & n;
        a ^= n;
        let c_bc = b & c_ab;
        b ^= c_ab;
        c |= c_bc;

        self.get_node(NodeData::Leaf((b & !c) & (a | input)))
    }

    /// Calculates the next state for a Branch node using 9-way decomposition.
    fn calc_branch(
        &mut self,
        nw: &Arc<Node>,
        ne: &Arc<Node>,
        sw: &Arc<Node>,
        se: &Arc<Node>,
        level: u8,
    ) -> Arc<Node> {
        if level == 4 {
            return self.calc_level_4_grid(nw, ne, sw, se, 4);
        }

        let n00 = nw.clone();
        let n01 = self.centered_horizontal(nw, ne);
        let n02 = ne.clone();

        let n10 = self.centered_vertical(nw, sw);
        let n11 = self.centered_sub(nw, ne, sw, se);
        let n12 = self.centered_vertical(ne, se);

        let n20 = sw.clone();
        let n21 = self.centered_horizontal(sw, se);
        let n22 = se.clone();

        let r00 = self.evolve(n00);
        let r01 = self.evolve(n01);
        let r02 = self.evolve(n02);
        let r10 = self.evolve(n10);
        let r11 = self.evolve(n11);
        let r12 = self.evolve(n12);
        let r20 = self.evolve(n20);
        let r21 = self.evolve(n21);
        let r22 = self.evolve(n22);

        let q_nw = self.join(r00.clone(), r01.clone(), r10.clone(), r11.clone());
        let q_ne = self.join(r01.clone(), r02.clone(), r11.clone(), r12.clone());
        let q_sw = self.join(r10.clone(), r11.clone(), r20.clone(), r21.clone());
        let q_se = self.join(r11, r12, r21, r22);

        let final_nw = self.evolve(q_nw);
        let final_ne = self.evolve(q_ne);
        let final_sw = self.evolve(q_sw);
        let final_se = self.evolve(q_se);

        self.join(final_nw, final_ne, final_sw, final_se)
    }

    /// Extracts the centered quarter node from 4 neighboring nodes.
    fn centered_sub(
        &mut self,
        nw: &Arc<Node>,
        ne: &Arc<Node>,
        sw: &Arc<Node>,
        se: &Arc<Node>,
    ) -> Arc<Node> {
        match (&nw.data, &ne.data, &sw.data, &se.data) {
            (
                NodeData::Leaf(nw_bits),
                NodeData::Leaf(ne_bits),
                NodeData::Leaf(sw_bits),
                NodeData::Leaf(se_bits),
            ) => self.centered_bits(*nw_bits, *ne_bits, *sw_bits, *se_bits),
            (
                NodeData::Branch { se: nw_se, .. },
                NodeData::Branch { sw: ne_sw, .. },
                NodeData::Branch { ne: sw_ne, .. },
                NodeData::Branch { nw: se_nw, .. },
            ) => self.join(nw_se.clone(), ne_sw.clone(), sw_ne.clone(), se_nw.clone()),
            _ => panic!("Mismatched node levels in centered_sub"),
        }
    }

    /// Extracts the horizontally centered half from two nodes.
    fn centered_horizontal(&mut self, left: &Arc<Node>, right: &Arc<Node>) -> Arc<Node> {
        match (&left.data, &right.data) {
            (NodeData::Leaf(l_bits), NodeData::Leaf(r_bits)) => {
                let mut res = 0u64;
                for y in 0..8 {
                    for x in 4..8 {
                        if (l_bits >> (y * 8 + x)) & 1 == 1 {
                            res |= 1 << (y * 8 + (x - 4));
                        }
                    }
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
            _ => panic!("Mismatched levels in centered_horizontal"),
        }
    }

    /// Extracts the vertically centered half from two nodes.
    fn centered_vertical(&mut self, top: &Arc<Node>, bottom: &Arc<Node>) -> Arc<Node> {
        match (&top.data, &bottom.data) {
            (NodeData::Leaf(t_bits), NodeData::Leaf(b_bits)) => {
                let mut res = 0u64;
                for y in 4..8 {
                    for x in 0..8 {
                        if (t_bits >> (y * 8 + x)) & 1 == 1 {
                            res |= 1 << ((y - 4) * 8 + x);
                        }
                    }
                }
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
            _ => panic!("Mismatched levels in centered_vertical"),
        }
    }

    /// Extracts the center 8x8 bits from four 8x8 Leaf nodes (forming a 16x16 grid).
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

    /// Optimized calculation for Level 4 nodes (16x16 grid composed of 4 leaves).
    /// Uses packed `u64` operations to simulate the grid efficiently.
    fn calc_level_4_grid(
        &mut self,
        nw: &Arc<Node>,
        ne: &Arc<Node>,
        sw: &Arc<Node>,
        se: &Arc<Node>,
        steps: usize,
    ) -> Arc<Node> {
        let (
            NodeData::Leaf(nw_bits),
            NodeData::Leaf(ne_bits),
            NodeData::Leaf(sw_bits),
            NodeData::Leaf(se_bits),
        ) = (&nw.data, &ne.data, &sw.data, &se.data)
        else {
            panic!("Level 4 children must be leaves");
        };

        // Assembly: Pack 4x 8x8 quadrants into 4x u64 blocks
        let mut b0 = self.zip_quadrants(*nw_bits, *ne_bits, 0); // Rows 0-3
        let mut b1 = self.zip_quadrants(*nw_bits, *ne_bits, 32); // Rows 4-7
        let mut b2 = self.zip_quadrants(*sw_bits, *se_bits, 0); // Rows 8-11
        let mut b3 = self.zip_quadrants(*sw_bits, *se_bits, 32); // Rows 12-15

        // Simulation Loop
        for _ in 0..steps {
            let n0 = self.step_4_rows(b0, 0, b1); // Top block
            let n1 = self.step_4_rows(b1, b0, b2); // Mid-Top
            let n2 = self.step_4_rows(b2, b1, b3); // Mid-Bot
            let n3 = self.step_4_rows(b3, b2, 0); // Bot block

            b0 = n0;
            b1 = n1;
            b2 = n2;
            b3 = n3;
        }

        // Disassembly: Extract Center 8x8
        let center_top = self.compress_center(b1);
        let center_bot = self.compress_center(b2);
        let result = center_top | (center_bot << 32);

        self.get_node(NodeData::Leaf(result))
    }

    /// Runs the SWAR Adder on 4 rows (packed in u64) simultaneously.
    fn step_4_rows(&mut self, curr: u64, up_block: u64, down_block: u64) -> u64 {
        // Vertical Neighbors
        // "Up" from Row 1 is Row 0. "Up" from Row 0 is last row of up_block.
        let u = (curr << 16) | (up_block >> 48);
        let d = (curr >> 16) | (down_block << 48);

        // Horizontal Neighbors (Masks prevent wrapping rows)
        const MASK_L: u64 = 0x7FFF7FFF7FFF7FFF;
        const MASK_R: u64 = 0xFFFEFFFEFFFEFFFE;

        let l = (curr >> 1) & MASK_L;
        let r = (curr << 1) & MASK_R;
        let ul = (u >> 1) & MASK_L;
        let ur = (u << 1) & MASK_R;
        let dl = (d >> 1) & MASK_L;
        let dr = (d << 1) & MASK_R;

        // Adder Tree
        let mut a = 0;
        let mut b = 0;
        let mut c = 0;

        let neighbors = [l, r, u, d, ul, ur, dl, dr];

        for n in neighbors {
            let c_ab = a & n;
            a ^= n;
            let c_bc = b & c_ab;
            b ^= c_ab;
            c |= c_bc;
        }

        (b & !c) & (a | curr)
    }

    /// Interleaves 4 bytes from left and right to create 4x 16-bit rows.
    /// `shift`: 0 for lower half of input, 32 for upper half.
    fn zip_quadrants(&mut self, left: u64, right: u64, shift: usize) -> u64 {
        let l_part = left >> shift;
        let r_part = right >> shift;

        let l0 = l_part & 0xFF;
        let l1 = (l_part >> 8) & 0xFF;
        let l2 = (l_part >> 16) & 0xFF;
        let l3 = (l_part >> 24) & 0xFF;

        let r0 = r_part & 0xFF;
        let r1 = (r_part >> 8) & 0xFF;
        let r2 = (r_part >> 16) & 0xFF;
        let r3 = (r_part >> 24) & 0xFF;

        let row0 = l0 | (r0 << 8);
        let row1 = l1 | (r1 << 8);
        let row2 = l2 | (r2 << 8);
        let row3 = l3 | (r3 << 8);

        row0 | (row1 << 16) | (row2 << 32) | (row3 << 48)
    }

    /// Extracts bits 4..11 from each 16-bit row and packs them into a 32-bit result.
    fn compress_center(&mut self, block: u64) -> u64 {
        let r0 = (block >> 4) & 0xFF;
        let r1 = (block >> (16 + 4)) & 0xFF;
        let r2 = (block >> (32 + 4)) & 0xFF;
        let r3 = (block >> (48 + 4)) & 0xFF;

        r0 | (r1 << 8) | (r2 << 16) | (r3 << 24)
    }
}
