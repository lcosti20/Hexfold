// Shows a portal's captured view at the fragment's own screen position.
import bevy_pbr::render::{forward_io::VertexOutput, mesh_view_bindings::view};

// x: 1 = flat pink, y: 1 = flip the image left-right
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var portal_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var portal_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    if (params.x > 0.5) {
        // The end of the recursion: a dim violet void (Unreal's plugin draws flat pink here).
        return vec4<f32>(0.10, 0.08, 0.14, 1.0);
    }
    var uv = (in.position.xy - view.viewport.xy) / view.viewport.zw;
    if (params.y > 0.5) {
        uv.x = 1.0 - uv.x;
    }
    return vec4<f32>(textureSampleLevel(portal_texture, portal_sampler, uv, 0.0).rgb, 1.0);
}
