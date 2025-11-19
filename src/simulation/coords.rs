use super::chunk::CHUNK_SIZE;
use bevy::prelude::*;

// Die einzige Stelle, an der wir die Zellgröße definieren!
pub const CELL_WIDTH: f32 = 20.0;

/// Konvertiert eine Welt-Koordinate (Maus/Screen) in eine Grid-Koordinate.
/// Nutzt floor(), damit auch negative Koordinaten korrekt landen (-0.5 -> -1).
#[inline(always)]
pub fn world_to_cell(world_pos: Vec2) -> IVec2 {
    IVec2::new(
        (world_pos.x / CELL_WIDTH).floor() as i32,
        (world_pos.y / CELL_WIDTH).floor() as i32,
    )
}

/// Berechnet die Welt-Position (Zentrum für Gizmos/Transform) einer Zelle.
#[inline(always)]
pub fn cell_to_world(grid_pos: IVec2) -> Vec2 {
    Vec2::new(
        grid_pos.x as f32 * CELL_WIDTH + (CELL_WIDTH / 2.0),
        grid_pos.y as f32 * CELL_WIDTH + (CELL_WIDTH / 2.0),
    )
}

/// Berechnet die Welt-Position (Zentrum für Gizmos) eines ganzen Chunks.
#[inline(always)]
pub fn chunk_to_world(chunk_pos: IVec2) -> Vec2 {
    let chunk_pixel_size = CHUNK_SIZE as f32 * CELL_WIDTH;

    // Basis-Koordinate (unten links)
    let base_x = chunk_pos.x as f32 * chunk_pixel_size;
    let base_y = chunk_pos.y as f32 * chunk_pixel_size;

    // Zentrum berechnen
    Vec2::new(
        base_x + (chunk_pixel_size / 2.0),
        base_y + (chunk_pixel_size / 2.0),
    )
}

/// Gibt die Größe eines Chunks in Welt-Einheiten zurück (für Gizmos).
#[inline(always)]
pub fn chunk_world_size() -> Vec2 {
    Vec2::splat(CHUNK_SIZE as f32 * CELL_WIDTH)
}
