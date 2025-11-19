use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::simulation::chunk::BitChunk;
use crate::simulation::coords::{chunk_to_world, chunk_world_size};
use crate::simulation::universe::Universe;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::tasks::ComputeTaskPool;
use bevy::window::PrimaryWindow;

pub fn tick_universe(mut universe: ResMut<Universe>) {
    let start_total = Instant::now();

    // --- Collect ---
    let sim_keys = collect_simulation_set(&universe);
    if sim_keys.is_empty() {
        return;
    }

    // --- Compute ---
    let start_compute = Instant::now();
    let next_chunks = Arc::new(Mutex::new(HashMap::new()));
    let universe_ref = &*universe;
    let pool = ComputeTaskPool::get();

    pool.scope(|s| {
        for &chunk_pos in &sim_keys {
            let next_chunks_clone = next_chunks.clone();

            s.spawn(async move {
                let empty = BitChunk::new();

                // Wir holen uns ALLE Nachbarn.
                // Grid Layout:
                // 2,0  2,1  2,2 (Oben)
                // 1,0  1,1  1,2 (Mitte)
                // 0,0  0,1  0,2 (Unten)
                // neighbors[y][x]
                let n = get_neighbor_refs(universe_ref, chunk_pos, &empty);

                // Center
                let c = n[1][1];

                // Wir rufen die optimierte Funktion auf
                // Parameter: North, South, West, East
                // Und die Ecken: NW, NE, SW, SE (als einzelne Bits oder wir übergeben die chunks)

                // Um die API einfach zu halten, lassen wir `step_optimized` die bits extrahieren.
                // Dafür erweitern wir BitChunk::step_bitwise in chunk.rs,
                // aber hier machen wir es manuell mit den Refs:

                // Ecken-Bits extrahieren (MSB/LSB von Ecken-Chunks)
                // NW (Top-Left) Chunk: Wir brauchen das Pixel unten rechts (x=63, y=0)
                let _nw_bit = (n[2][0].data[0] >> 63) & 1;
                let _ne_bit = (n[2][2].data[0] >> 0) & 1;
                let _sw_bit = (n[0][0].data[63] >> 63) & 1;
                let _se_bit = (n[0][2].data[63] >> 0) & 1;

                // Das ist etwas fummelig.
                // Besser: Wir schreiben eine Wrapper-Funktion im Chunk, die [BitChunk; 9] nimmt.
                let (next_chunk, alive) = c.step_bitwise_9(n);

                if alive {
                    let mut map = next_chunks_clone.lock().unwrap();
                    map.insert(chunk_pos, next_chunk);
                }
            });
        }
    });
    let compute_duration = start_compute.elapsed();

    // --- Merge ---
    let final_map = Arc::try_unwrap(next_chunks).unwrap().into_inner().unwrap();
    universe.chunks = final_map;

    let total = start_total.elapsed();
    if total.as_micros() > 100 {
        println!("Tick: {:?} (Compute: {:?})", total, compute_duration);
    }
}

// --- Helper ---
fn get_neighbor_refs<'a>(
    universe: &'a Universe,
    center_pos: IVec2,
    empty: &'a BitChunk,
) -> [[&'a BitChunk; 3]; 3] {
    let mut refs = [[empty; 3]; 3];
    for dy in -1..=1 {
        for dx in -1..=1 {
            let pos = center_pos + IVec2::new(dx, dy);
            if let Some(chunk) = universe.chunks.get(&pos) {
                refs[(dy + 1) as usize][(dx + 1) as usize] = chunk;
            }
        }
    }
    refs
}

fn collect_simulation_set(universe: &Universe) -> HashSet<IVec2> {
    let mut sim_set = HashSet::with_capacity(universe.chunks.len() * 2);
    for (pos, chunk) in &universe.chunks {
        sim_set.insert(*pos);

        // Ränder prüfen (Optimiert: Ganze Zeile auf einmal prüfen != 0)
        if chunk.data[0] > 0 {
            sim_set.insert(*pos + IVec2::new(0, -1));
        } // Unten
        if chunk.data[63] > 0 {
            sim_set.insert(*pos + IVec2::new(0, 1));
        } // Oben

        let mut left = false;
        let mut right = false;
        for y in 0..64 {
            if chunk.data[y] & 1 == 1 {
                left = true;
            }
            if chunk.data[y] & (1 << 63) != 0 {
                right = true;
            }
        }
        if left {
            sim_set.insert(*pos + IVec2::new(-1, 0));
        }
        if right {
            sim_set.insert(*pos + IVec2::new(1, 0));
        }

        // Ecken
        if (chunk.data[0] & 1) != 0 {
            sim_set.insert(*pos + IVec2::new(-1, -1));
        }
        if (chunk.data[0] & (1 << 63)) != 0 {
            sim_set.insert(*pos + IVec2::new(1, -1));
        }
        if (chunk.data[63] & 1) != 0 {
            sim_set.insert(*pos + IVec2::new(-1, 1));
        }
        if (chunk.data[63] & (1 << 63)) != 0 {
            sim_set.insert(*pos + IVec2::new(1, 1));
        }
    }
    sim_set
}

pub fn _draw_chunks_debug(
    universe: Res<Universe>,
    mut gizmos: Gizmos,
    q_camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
) {
    let visible_rect = if let (Ok((camera, camera_transform)), Ok(window)) =
        (q_camera.single(), q_window.single())
    {
        let min_dest = Vec2::ZERO;
        let max_dest = Vec2::new(window.width(), window.height());

        if let (Some(top_left), Some(bottom_right)) = (
            camera.viewport_to_world_2d(camera_transform, min_dest).ok(),
            camera.viewport_to_world_2d(camera_transform, max_dest).ok(),
        ) {
            Some(Rect::from_corners(top_left, bottom_right))
        } else {
            None
        }
    } else {
        None
    };

    let chunk_size_vec = chunk_world_size();

    for (chunk_pos, _chunk) in &universe.chunks {
        let chunk_center = chunk_to_world(*chunk_pos);

        if let Some(rect) = visible_rect {
            let chunk_rect = Rect::from_center_size(chunk_center, chunk_size_vec);
            if rect.intersect(chunk_rect).is_empty() {
                continue;
            }
        }

        gizmos.rect_2d(
            Isometry2d::from_translation(chunk_center),
            chunk_size_vec,
            Color::oklch(0.7, 0.1232, 140.34),
        );

        // for y in 0..CHUNK_SIZE {
        //     if chunk.data[y as usize] == 0 {
        //         continue;
        //     }

        //     for x in 0..CHUNK_SIZE {
        //         if chunk.get(x, y) {
        //             let global_pos =
        //                 IVec2::new(chunk_pos.x * CHUNK_SIZE + x, chunk_pos.y * CHUNK_SIZE + y);

        //             gizmos.rect_2d(
        //                 Isometry2d::from_translation(cell_to_world(global_pos)),
        //                 Vec2::splat(CELL_WIDTH * 0.9),
        //                 Color::WHITE,
        //             );
        //         }
        //     }
        // }
    }
}
