#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct BitChunkMaterial {
    color_alive: vec4<f32>,
    color_dead: vec4<f32>,
};

@group(2) @binding(0) var<uniform> material: BitChunkMaterial;
@group(2) @binding(1) var data_texture: texture_2d<u32>;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dims = textureDimensions(data_texture);

    // Map UV to pixel coordinates
    let x = clamp(u32(in.uv.x * f32(dims.x)), 0u, dims.x - 1u);
    let y = clamp(u32((1.0 - in.uv.y) * f32(dims.y)), 0u, dims.y - 1u);

    // Load the density value (0 to 255)
    let raw_value = textureLoad(data_texture, vec2<u32>(x, y), 0).r;

    // Normalize the integer to a float factor (0.0 to 1.0)
    let t = f32(raw_value) / 255.0;

    // Linear Interpolation (Lerp)
    return mix(material.color_dead, material.color_alive, t);
}
