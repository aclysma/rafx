#include "../util/taa_jitter.glsl"

// @[semantic("POSITION")]
layout (location = 0) in vec3 in_pos;

// @[semantic("NORMAL")]
layout (location = 1) in vec3 in_normal;

// w component is a sign value (-1 or +1) indicating handedness of the tangent basis
// see GLTF spec for more info
// @[semantic("TANGENT")]
layout (location = 2) in vec3 in_tangent;

// @[semantic("BINORMAL")]
layout (location = 3) in vec3 in_binormal;

// @[semantic("TEXCOORD")]
layout (location = 4) in vec2 in_uv;

// Do all math in view space so that it is more easily portable to deferred/clustered
// forward rendering (vs = view space)
layout (location = 0) out vec3 out_position_vs;
layout (location = 1) out vec3 out_normal_vs;
layout (location = 2) out vec3 out_tangent_vs;
layout (location = 3) out vec3 out_binormal_vs;
layout (location = 4) out vec2 out_uv;

// for shadows
layout (location = 5) out vec4 out_position_ws;
layout (location = 6) out mat3 out_model_view;
//layout (location = 7) out mat3 out_model_view;
//layout (location = 8) out mat3 out_model_view;
layout (location = 9) flat out uint out_instance_index;

invariant gl_Position;

void pbr_main() {
    // WARNING: The operations that contribute to computing gl_Position must be kept in EXACT sync with other places to
    // ensure deterministic values are generated between depth prepass and rendering
#ifdef PLATFORM_DX12
    uint instance_index = push_constants.instance_offset;
    // HACK: GBV seems to cause instance_index to be bad values, this protects from causing a crash
    if (instance_index > all_draw_data.count) {
        instance_index = 0;
    }
#else
    uint instance_index = gl_InstanceIndex;
#endif

    DrawData draw_data = all_draw_data.draw_data[instance_index];
    mat4 model_matrix = all_transforms.transforms[draw_data.transform_index].model_matrix;
    mat4 model_view_proj = per_view_data.view_proj * model_matrix;

    vec4 position_clip = model_view_proj * vec4(in_pos, 1.0);
    gl_Position = add_jitter(position_clip, per_view_data.jitter_amount);
    // End deterministic path

    vec2 viewport_size = vec2(per_view_data.viewport_width, per_view_data.viewport_height);
    mat4 model_view = per_view_data.view * model_matrix;
    out_position_vs = (model_view * vec4(in_pos, 1.0)).xyz;

    // This can be skipped if just using rotation/uniform scale. Required for non-uniform scale/shear
    mat3 normalMatrix = transpose(inverse(mat3(model_view)));
    vec3 n = normalize(normalMatrix * in_normal);
    vec3 t = normalize(normalMatrix * in_tangent);
    vec3 b = normalize(normalMatrix * in_binormal);

    out_uv = in_uv;
    out_normal_vs = n;
    out_tangent_vs = t;
    out_binormal_vs = b;

    // Used to sample the shadow map
    out_position_ws = model_matrix * vec4(in_pos, 1.0);

    out_model_view = mat3(model_view);
    out_instance_index = instance_index;

}