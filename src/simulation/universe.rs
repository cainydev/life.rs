use crate::simulation::chunk::{BitChunk, CHUNK_SIZE};
use bevy::{platform::collections::HashMap, prelude::*};

#[derive(Resource, Default)]
pub struct Universe {
    pub chunks: HashMap<IVec2, BitChunk>,
}

impl Universe {
    /// Liest eine Zelle anhand globaler Welt-Koordinaten
    pub fn _get_cell(&self, global_pos: IVec2) -> bool {
        let chunk_idx = IVec2::new(
            global_pos.x.div_euclid(CHUNK_SIZE),
            global_pos.y.div_euclid(CHUNK_SIZE),
        );

        let local_x = global_pos.x.rem_euclid(CHUNK_SIZE);
        let local_y = global_pos.y.rem_euclid(CHUNK_SIZE);

        if let Some(chunk) = self.chunks.get(&chunk_idx) {
            chunk.get(local_x, local_y)
        } else {
            false
        }
    }

    /// Setzt eine Zelle (erstellt Chunk falls nötig)
    pub fn set_cell(&mut self, global_pos: IVec2, value: bool) {
        let chunk_idx = IVec2::new(
            global_pos.x.div_euclid(CHUNK_SIZE),
            global_pos.y.div_euclid(CHUNK_SIZE),
        );

        let local_x = global_pos.x.rem_euclid(CHUNK_SIZE);
        let local_y = global_pos.y.rem_euclid(CHUNK_SIZE);

        let chunk = self.chunks.entry(chunk_idx).or_insert_with(BitChunk::new);
        chunk.set(local_x, local_y, value);
    }
}
