use crate::simulation::chunk::{BitChunk, CHUNK_SIZE};
use crate::simulation::coords::{chunk_to_world, chunk_world_size};
use crate::simulation::universe::Universe;
use bevy::asset::RenderAssetUsages;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow; // <--- Wichtig für Culling

#[derive(Resource, Default)]
pub struct ChunkRenderCache {
    pub entities: HashMap<IVec2, (Entity, Handle<Image>)>,
}

pub fn render_chunks_optimized(
    mut commands: Commands,
    universe: Res<Universe>,
    mut cache: ResMut<ChunkRenderCache>,
    mut images: ResMut<Assets<Image>>,
    // Queries für Kamera und Fenster (für Culling)
    q_camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
) {
    // --- 1. Sichtbaren Bereich berechnen (Culling) ---
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
            None // Kamera sieht nichts (oder Fehler), also rendern wir sicherheitshalber nichts oder alles?
            // None führt unten dazu, dass alles gerendert wird (Safety Fallback).
        }
    } else {
        None
    };

    let chunk_size_vec = chunk_world_size();

    // --- 2. Cleanup: Chunks löschen, die es in der Simulation nicht mehr gibt ---
    cache.entities.retain(|pos, (entity, _)| {
        if !universe.chunks.contains_key(pos) {
            commands.entity(*entity).despawn();
            return false;
        }
        true
    });

    let universe_changed = universe.is_changed();

    // --- 3. Rendering Loop mit Culling ---
    for (pos, chunk) in &universe.chunks {
        // AABB (Axis Aligned Bounding Box) für den Chunk berechnen
        let chunk_center = chunk_to_world(*pos);
        let chunk_rect = Rect::from_center_size(chunk_center, chunk_size_vec);

        // Culling Check: Ist der Chunk sichtbar?
        let is_visible = if let Some(rect) = visible_rect {
            !rect.intersect(chunk_rect).is_empty()
        } else {
            true // Fallback: Alles zeigen wenn Kamera-Check fehlschlägt
        };

        if is_visible {
            // --- A. Chunk ist sichtbar ---

            // Prüfen, ob er schon existiert (im Cache ist)
            // Wir clonen das Handle nur kurz, um den Borrow Checker glücklich zu machen
            let cached_handle = cache.entities.get(pos).map(|(_, h)| h.clone());

            if let Some(image_handle) = cached_handle {
                // EXISTIERT BEREITS: Nur Textur updaten, wenn Simulation lief
                if universe_changed {
                    if let Some(image) = images.get_mut(&image_handle) {
                        update_texture_data_fast(image, chunk);
                    }
                }
            } else {
                // EXISTIERT NICHT: Neu erstellen (Spawn)
                let mut image = create_chunk_image();
                update_texture_data_fast(&mut image, chunk);
                let handle = images.add(image);

                let entity = commands
                    .spawn((
                        Sprite {
                            image: handle.clone(),
                            custom_size: Some(chunk_size_vec),
                            ..default()
                        },
                        // Z=-1.0 damit Gizmos und Maus drüber liegen
                        Transform::from_translation(chunk_center.extend(-1.0)),
                    ))
                    .id();

                cache.entities.insert(*pos, (entity, handle));
            }
        } else {
            // --- B. Chunk ist NICHT sichtbar ---

            // Wenn er noch im Cache ist -> Löschen (Despawn) um Ressourcen zu sparen
            if let Some((entity, _)) = cache.entities.remove(pos) {
                commands.entity(entity).despawn();
            }
        }
    }
}

fn create_chunk_image() -> Image {
    let size = Extent3d {
        width: CHUNK_SIZE as u32,
        height: CHUNK_SIZE as u32,
        depth_or_array_layers: 1,
    };

    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );

    image.sampler = bevy::image::ImageSampler::nearest();
    image
}

// High-Performance Texture Writer
fn update_texture_data_fast(image: &mut Image, chunk: &BitChunk) {
    let data = match &mut image.data {
        Some(d) => d,
        None => return,
    };

    const COLOR_ALIVE: [u8; 4] = [255, 255, 255, 255];
    const COLOR_DEAD: [u8; 4] = [0, 0, 0, 0];

    for (row_idx, &row_bits) in chunk.data.iter().enumerate() {
        // Y-Flip beachten (Texturkoordinaten vs Gridkoordinaten)
        let tex_y = CHUNK_SIZE as usize - 1 - row_idx;
        let row_start_idx = tex_y * (CHUNK_SIZE as usize) * 4;

        // Optimization: Wenn Zeile leer, könnten wir skippen (aber Alpha muss 0 sein!)
        // Da wir hier "overwriten", schreiben wir einfach alles.

        for bit in 0..64 {
            let is_alive = (row_bits >> bit) & 1 == 1;
            let pixel_idx = row_start_idx + (bit * 4);

            if is_alive {
                data[pixel_idx..pixel_idx + 4].copy_from_slice(&COLOR_ALIVE);
            } else {
                data[pixel_idx..pixel_idx + 4].copy_from_slice(&COLOR_DEAD);
            }
        }
    }
}
