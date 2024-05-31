#import bevy_pbr::{
    pbr_deferred_functions::deferred_output,
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{VertexOutput, FragmentOutput},
    prepass_utils,
    view_transformations::{depth_ndc_to_view_z, frag_coord_to_ndc},
}
#import bevy_render::globals::Globals

fn depth_fade(frag_coord: vec4<f32>, distance: f32) -> f32 {
    let depth = depth_ndc_to_view_z(prepass_utils::prepass_depth(frag_coord, 0u));
    return saturate((depth_ndc_to_view_z(frag_coord.z) - depth) / distance);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let offset = 0.9;
    let intersection_intensity = 10.0;

    let shallow_water = vec4(0.5, 0.75, 1.0, 0.25);
    let deep_water = vec4(0.0, 0.0, 0.1, 0.75);

    let foam_amount = 0.2;
    let foam_cutoff = 1.0;
    let foam = depth_fade(in.position, foam_amount) * foam_cutoff;

    if foam > 0.25 {
        discard;
    }
    return vec4(1.0 - foam);
}
