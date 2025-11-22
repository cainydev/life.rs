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

    let x = clamp(u32(in.uv.x * f32(dims.x)), 0u, dims.x - 1u);
    let y = clamp(u32((1.0 - in.uv.y) * f32(dims.y)), 0u, dims.y - 1u);

    let state = textureLoad(data_texture, vec2<u32>(x, y), 0).r;

    if (state > 0u) {
        return material.color_alive;
    } else {
        return material.color_dead;
    }
}
