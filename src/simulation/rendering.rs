use crate::plugins::stats_boards::StatsBoard;
use crate::simulation::chunk::{BitChunk, CHUNK_SIZE};
use crate::simulation::coords::{chunk_to_world, chunk_world_size};
use crate::simulation::universe::Universe;
use bevy::asset::RenderAssetUsages;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d}; // Or MeshMaterial2d in Bevy 0.15
use bevy::window::PrimaryWindow;
use bytemuck::checked::cast_slice;

#[derive(Resource, Default)]
pub struct ChunkRenderCache {
    // We cache the Entity and the handles
    pub entities: HashMap<IVec2, (Entity, Handle<BitChunkMaterial>, Handle<Image>)>,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct BitChunkMaterial {
    #[uniform(0)]
    pub color_alive: Vec4,
    #[uniform(0)]
    pub color_dead: Vec4,
    #[texture(1, sample_type = "u_int")]
    pub image: Handle<Image>,
}

impl Material2d for BitChunkMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/chunk_shader.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

pub fn render_chunks(
    mut commands: Commands,
    universe: Res<Universe>,
    mut cache: ResMut<ChunkRenderCache>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BitChunkMaterial>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    mut stats: ResMut<StatsBoard>,
) {
    // --- 1. Culling ---
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

    cache.entities.retain(|pos, (entity, _, _)| {
        if !universe.chunks.contains_key(pos) {
            commands.entity(*entity).despawn();
            return false;
        }
        true
    });

    if !universe.is_changed() {
        return;
    }

    let num_total = &universe.chunks.len();
    let mut num_visible = 0;
    let mut num_changed = 0;
    let mut num_created = 0;

    for (pos, chunk) in &universe.chunks {
        let chunk_center = chunk_to_world(*pos);
        let chunk_rect = Rect::from_center_size(chunk_center, chunk_size_vec);

        let is_visible = if let Some(rect) = visible_rect {
            !rect.intersect(chunk_rect).is_empty()
        } else {
            true
        };

        if is_visible {
            num_visible += 1;

            if let Some((_, material_handle, image_handle)) = cache.entities.get(pos) {
                num_changed += 1;

                if let Some(image) = images.get_mut(image_handle) {
                    if let Some(data_vec) = &mut image.data {
                        data_vec.copy_from_slice(cast_slice(&chunk.data));
                    }
                }

                let _ = materials.get_mut(material_handle);
            } else {
                num_created += 1;
                let texture_handle = create_gpu_data_texture(&mut images, chunk);

                let material = BitChunkMaterial {
                    color_alive: Vec4::new(1.0, 1.0, 1.0, 1.0),
                    color_dead: Vec4::new(0.0, 0.0, 0.0, 0.0),
                    image: texture_handle.clone(),
                };

                let material_handle = materials.add(material);
                let mesh_handle = meshes.add(Rectangle::from_size(chunk_size_vec));

                let entity = commands
                    .spawn((
                        Mesh2d(mesh_handle),
                        MeshMaterial2d(material_handle.clone()),
                        Transform::from_translation(chunk_center.extend(1.0)),
                    ))
                    .id();

                cache
                    .entities
                    .insert(*pos, (entity, material_handle, texture_handle));
            }
        }
    }

    stats.insert("total chunks", num_total);
    stats.insert("visible chunks", num_visible);
    stats.insert("changed chunks", num_changed);
    stats.insert("created chunks", num_created);
}

fn create_gpu_data_texture(images: &mut Assets<Image>, chunk: &BitChunk) -> Handle<Image> {
    let size = Extent3d {
        width: (CHUNK_SIZE * 2) as u32,
        height: 1,
        depth_or_array_layers: 1,
    };

    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        cast_slice(&chunk.data),
        TextureFormat::R32Uint,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );

    image.sampler = bevy::image::ImageSampler::nearest();
    images.add(image)
}
