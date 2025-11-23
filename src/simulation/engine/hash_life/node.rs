use std::{
    hash::{Hash, Hasher},
    sync::{Arc, OnceLock},
};

#[derive(Clone, Hash)]
pub enum NodeData {
    Leaf(u64),
    Branch {
        nw: Arc<Node>,
        ne: Arc<Node>,
        sw: Arc<Node>,
        se: Arc<Node>,
        level: u8,
    },
}

impl PartialEq for NodeData {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (NodeData::Leaf(a), NodeData::Leaf(b)) => a == b,
            (
                NodeData::Branch {
                    nw: nw1,
                    ne: ne1,
                    sw: sw1,
                    se: se1,
                    level: l1,
                },
                NodeData::Branch {
                    nw: nw2,
                    ne: ne2,
                    sw: sw2,
                    se: se2,
                    level: l2,
                },
            ) => {
                l1 == l2
                    && Arc::ptr_eq(nw1, nw2)
                    && Arc::ptr_eq(ne1, ne2)
                    && Arc::ptr_eq(sw1, sw2)
                    && Arc::ptr_eq(se1, se2)
            }
            _ => false,
        }
    }
}

impl Eq for NodeData {}

pub struct Node {
    pub data: NodeData,
    pub population: u64,
    pub hash: u64,

    /// Cached result for the standard Hashlife "Warp Speed" jump (2^(level-2) generations)
    pub result: OnceLock<Arc<Node>>,

    /// Cached result for exactly 1 generation
    pub result_step_1: OnceLock<Arc<Node>>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        if self.hash != other.hash {
            return false;
        }
        self.data == other.data
    }
}

impl Eq for Node {}

impl Hash for Node {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl Node {
    pub fn level(&self) -> u8 {
        match &self.data {
            NodeData::Leaf(_) => 3,
            NodeData::Branch { level, .. } => *level,
        }
    }
}
