use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d};
use bevy::window::PrimaryWindow;

use crate::simulation::view::SimulationView;

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<GridLayerMaterial>::default())
            // This system handles scaling and refreshing for EVERY pixel layer automatically
            .add_systems(PostUpdate, manage_pixel_layers);
    }
}

// --- 1. The Component & Bundle ---

/// Tag component for any entity that renders a pixel buffer.
/// Holds the image handle so we can refresh the material automatically.
#[derive(Component)]
pub struct PixelLayer {
    pub image_handle: Handle<Image>,
}

/// Spawn this bundle to create a fully managed fullscreen drawing layer.
#[derive(Bundle)]
pub struct PixelLayerBundle {
    pub layer: PixelLayer,
    pub mesh: Mesh2d,
    pub material: MeshMaterial2d<GridLayerMaterial>,
    pub transform: Transform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
}

impl PixelLayerBundle {
    pub fn new(
        images: &mut Assets<Image>,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<GridLayerMaterial>,
        z_index: f32,
        color_alive: Vec4,
        color_dead: Vec4,
    ) -> Self {
        let width = 32;
        let height = 32;

        let size = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let mut image = Image::new_fill(
            size,
            TextureDimension::D2,
            &vec![0u8; (width * height) as usize],
            TextureFormat::R8Uint,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );
        image.sampler = bevy::image::ImageSampler::nearest();
        let image_handle = images.add(image);

        let material_handle = materials.add(GridLayerMaterial {
            color_alive,
            color_dead,
            image: image_handle.clone(),
        });

        Self {
            layer: PixelLayer { image_handle },
            mesh: Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))),
            material: MeshMaterial2d(material_handle),
            transform: Transform::from_xyz(0.0, 0.0, z_index),
            visibility: Visibility::default(),
            inherited_visibility: InheritedVisibility::default(),
            view_visibility: ViewVisibility::default(),
        }
    }
}

// --- 2. The Infrastructure System ---

fn manage_pixel_layers(
    q_window: Query<&Window, With<PrimaryWindow>>,
    // Query ALL layers (Universe, Draw, etc.)
    mut q_layers: Query<(
        &mut Transform,
        &MeshMaterial2d<GridLayerMaterial>,
        &PixelLayer,
    )>,
    mut materials: ResMut<Assets<GridLayerMaterial>>,
) {
    let Ok(window) = q_window.single() else {
        return;
    };
    let width = window.width();
    let height = window.height();

    // Scale 1.0 -> Screen Dimensions
    let scale = Vec3::new(width, height, 1.0);

    for (mut transform, mat_handle, layer) in q_layers.iter_mut() {
        // 1. Auto-Scale the mesh to fit the window
        transform.scale = scale;

        // 2. Auto-Refresh the material (Fixes Bevy not updating texture content)
        if let Some(material) = materials.get_mut(&mat_handle.0) {
            material.image = layer.image_handle.clone();
        }
    }
}

// --- 3. Shared Resources ---

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GridLayerMaterial {
    #[uniform(0)]
    pub color_alive: Vec4,
    #[uniform(0)]
    pub color_dead: Vec4,
    #[texture(1, sample_type = "u_int")]
    pub image: Handle<Image>,
}

impl Material2d for GridLayerMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/chunk_shader.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

pub struct LayerViewport {
    pub screen_w: usize,
    pub screen_h: usize,
    pub min_x: f64,
    pub min_y: f64,
    pub scale: f64,
}

impl LayerViewport {
    pub fn new(window: &Window, view: &SimulationView) -> Option<Self> {
        let screen_w = window.physical_width() as usize;
        let screen_h = window.physical_height() as usize;
        if screen_w == 0 || screen_h == 0 {
            return None;
        }

        let world_w = window.width() as f64 / view.zoom as f64;
        let world_h = window.height() as f64 / view.zoom as f64;
        let min_x = view.center.x as f64 - (world_w / 2.0);
        let min_y = view.center.y as f64 - (world_h / 2.0);
        let scale = screen_w as f64 / world_w;

        Some(Self {
            screen_w,
            screen_h,
            min_x,
            min_y,
            scale,
        })
    }

    pub fn get_buffer<'a>(&self, image: &'a mut Image) -> &'a mut [u8] {
        let width = self.screen_w as u32;
        let height = self.screen_h as u32;
        if image.width() != width || image.height() != height {
            image.resize(Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            });
        }
        let len = self.screen_w * self.screen_h;
        if image.data.is_none() || image.data.as_ref().map(|d| d.len()).unwrap_or(0) != len {
            image.data = Some(vec![0u8; len]);
        }
        image.data.as_mut().unwrap()
    }

    pub fn get_world_rect(&self) -> Rect {
        let width = self.screen_w as f64 / self.scale;
        let height = self.screen_h as f64 / self.scale;
        Rect {
            min: Vec2::new(self.min_x as f32, self.min_y as f32),
            max: Vec2::new((self.min_x + width) as f32, (self.min_y + height) as f32),
        }
    }

    pub fn draw_cell(&self, buffer: &mut [u8], gx: i64, gy: i64, value: u8) {
        let screen_x = (gx as f64 - self.min_x) * self.scale;
        let screen_y = (gy as f64 - self.min_y) * self.scale;

        if screen_x >= self.screen_w as f64 || screen_y >= self.screen_h as f64 {
            return;
        }
        if screen_x + self.scale <= 0.0 || screen_y + self.scale <= 0.0 {
            return;
        }

        let start_x = screen_x.floor().max(0.0) as usize;
        let start_y = screen_y.floor().max(0.0) as usize;
        let end_x = (screen_x + self.scale).ceil().min(self.screen_w as f64) as usize;
        let end_y = (screen_y + self.scale).ceil().min(self.screen_h as f64) as usize;

        for y in start_y..end_y {
            let row_offset = y * self.screen_w;
            buffer[row_offset + start_x..row_offset + end_x].fill(value);
        }
    }
}
