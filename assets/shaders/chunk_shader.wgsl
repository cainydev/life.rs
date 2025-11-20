#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct BitChunkMaterial {
    color_alive: vec4<f32>,
    color_dead: vec4<f32>,
};

@group(2) @binding(0) var<uniform> material: BitChunkMaterial;
// Note: usampler, not sampler, because we are reading uints
@group(2) @binding(1) var data_texture: texture_2d<u32>;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // 1. Calculate which simulation row (0-63) we are on
    // (Flip Y if necessary depending on your coordinate system)
    let sim_y = i32((1.0 - in.uv.y) * 64.0);
    let sim_x = i32(in.uv.x * 64.0);

    // Safety clamp
    let row_idx = clamp(sim_y, 0, 63);
    let col_idx = clamp(sim_x, 0, 63);

    // 2. Fetch data from the 128x1 Texture
    // The Row Index (0-63) maps to Texture X coordinates.
    // Each row takes 2 pixels.
    // Start index = row_idx * 2

    let start_tex_x = row_idx * 2;

    var is_alive = false;

    if (col_idx < 32) {
        // --- LOWER 32 BITS ---
        // Read the first pixel for this row
        // Note: .r gives us the uint value directly
        let data_lower = textureLoad(data_texture, vec2<i32>(start_tex_x, 0), 0).r;

        // Check the bit
        is_alive = ((data_lower >> u32(col_idx)) & 1u) == 1u;
    } else {
        // --- UPPER 32 BITS ---
        // Read the second pixel (next neighbor)
        let data_upper = textureLoad(data_texture, vec2<i32>(start_tex_x + 1, 0), 0).r;

        // Adjust shift (subtract 32)
        is_alive = ((data_upper >> u32(col_idx - 32)) & 1u) == 1u;
    }

    if (is_alive) {
        return material.color_alive;
    }
    return material.color_dead;
}
