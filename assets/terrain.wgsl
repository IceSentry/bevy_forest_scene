#import bevy_pbr::{
    pbr_deferred_functions::deferred_output,
    pbr_fragment::pbr_input_from_standard_material,
    prepass_io::{VertexOutput, FragmentOutput},
    prepass_utils,
    mesh_view_bindings::view,
    pbr_functions,
    pbr_functions::SampleBias,
    pbr_bindings,
    lighting,
    parallax_mapping,
    pbr_types::{PbrInput, pbr_input_new},
}


struct TerrainMaterialSettings {
    max_steepness: f32,
}
@group(2) @binding(100) var<uniform> settings: TerrainMaterialSettings;

// #define USE_PARALLAX
// #define USE_TRIPLANAR

fn get_uv(uv: vec2f, Vt: vec3f) -> vec2f {
#ifdef USE_PARALLAX
    return parallax_mapping::parallaxed_uv(
        pbr_bindings::material.parallax_depth_scale,
        pbr_bindings::material.max_parallax_layer_count,
        pbr_bindings::material.max_relief_mapping_search_steps,
        uv,
        // Flip the direction of Vt to go toward the surface to make the
        // parallax mapping algorithm easier to understand and reason
        // about.
        -Vt,
    );
#else // USE_PARALLAX
    return uv;
#endif // USE_PARALLAX
}


fn triplanar_mapping(
    world_pos: vec4f,
    scale: f32,
    blend_axes: vec3f,
    bias: SampleBias,
    t: texture_2d<f32>,
    s: sampler,
    Vt: vec3f
) -> vec4f {
    let scaled_world_pos = world_pos / scale;
#ifdef USE_TRIPLANAR
    let x_projeciton = pbr_functions::sample_texture(
        t, s, scaled_world_pos.yz, bias
    ) * blend_axes.x;
    let y_projection = pbr_functions::sample_texture(
        t, s, scaled_world_pos.xz, bias
    ) * blend_axes.y;
    let z_projection = pbr_functions::sample_texture(
        t, s, scaled_world_pos.xy, bias
    ) * blend_axes.z;
    let base_color = x_projeciton + y_projection + z_projection;
    return base_color;
#else
    return pbr_functions::sample_texture(
        t, s, get_uv(scaled_world_pos.xz, Vt), bias
    );
#endif
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    // Create the PBR input.
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    // var pbr_input: PbrInput = pbr_input_new();

    let up = vec3(0.0, 1.0, 0.0);
    let steepness = length(cross(in.world_normal, up));
    pbr_input.material.base_color = mix(
        pbr_input.material.base_color,
        vec4(1.0, 0.0, 0.0, 1.0),
        saturate(steepness - settings.max_steepness)
    );

// #ifdef USE_PARALLAX
//     let V = pbr_input.V;
//     let N = in.world_normal;
//     let T = in.world_tangent.xyz;
//     let B = in.world_tangent.w * cross(N, T);
//     // Transform V from fragment to camera in world space to tangent space.
//     let Vt = vec3(dot(V, T), dot(V, B), dot(V, N));
// #else
//     let Vt = vec3(0.0);
// #endif

//     let double_sided = false;
//     pbr_input.frag_coord = in.position;
//     pbr_input.world_position = in.world_position;
//     pbr_input.world_normal = pbr_functions::prepare_world_normal(
//         in.world_normal,
//         double_sided,
//         is_front,
//     );

// // //     // pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;

//     // pbr_input.N = normalize(pbr_input.world_normal);

//     var blend_axes = abs(in.world_normal);
//     blend_axes /= blend_axes.x + blend_axes.y + blend_axes.z;

//     let scale = 25.0;

//     var bias: SampleBias;
//     bias.mip_bias = view.mip_bias;

//     // base_color
//     let base_color = triplanar_mapping(
//         in.world_position,
//         scale,
//         blend_axes,
//         bias,
//         pbr_bindings::base_color_texture,
//         pbr_bindings::base_color_sampler,
//         Vt
//     );
//     pbr_input.material.base_color = base_color;

//     // metallic_roughness
//     var metallic: f32 = pbr_bindings::material.metallic;
//     var perceptual_roughness: f32 = pbr_bindings::material.perceptual_roughness;
//     let roughness = lighting::perceptualRoughnessToRoughness(perceptual_roughness);

//     let metallic_roughness = triplanar_mapping(
//         in.world_position,
//         scale,
//         blend_axes,
//         bias,
//         pbr_bindings::metallic_roughness_texture,
//         pbr_bindings::base_color_sampler,
//         Vt
//     );
//     metallic *= metallic_roughness.b;
//     perceptual_roughness *= metallic_roughness.g;

//     pbr_input.material.metallic = metallic;
//     pbr_input.material.perceptual_roughness = perceptual_roughness;

//     // normal_map
//     let Nt = triplanar_mapping(
//         in.world_position,
//         scale,
//         blend_axes,
//         bias,
//         pbr_bindings::normal_map_texture,
//         pbr_bindings::normal_map_sampler,
//         Vt
//     ).rgb;
//     let TBN = pbr_functions::calculate_tbn_mikktspace(in.world_normal, in.world_tangent);
//     pbr_input.N = pbr_functions::apply_normal_mapping(
//         pbr_bindings::material.flags,
//         TBN,
//         double_sided,
//         is_front,
//         Nt,
//         view.mip_bias,
//     );

    // Send the rest to the deferred shader.
    return deferred_output(in, pbr_input);
}
