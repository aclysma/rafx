#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

//
// Per-Frame Pass
//
struct PointLight {
    vec3 position_world;
    vec3 position_view;
    vec4 color;
    float range;
    float intensity;
};

layout (set = 0, binding = 0) uniform PerFrameData {
    uint point_light_count;
    uint directional_light_count;
    uint spot_light_count;
    PointLight point_lights[16];
} per_frame_data;

layout (set = 0, binding = 1) uniform sampler smp;

//
// Per-Material Bindings
//
layout (set = 1, binding = 0) uniform MaterialData {
    vec4 base_color_factor;
    vec3 emissive_factor;
    float pad0;
    float metallic_factor;
    float roughness_factor;
    float normal_texture_scale;
    float occlusion_texture_strength;
    float alpha_cutoff;
    bool has_base_color_texture;
    bool has_metallic_roughness_texture;
    bool has_normal_texture;
    bool has_occlusion_texture;
    bool has_emissive_texture;
} material_data;

layout (set = 1, binding = 1) uniform texture2D base_color_texture;
layout (set = 1, binding = 2) uniform texture2D metallic_roughness_texture;
layout (set = 1, binding = 3) uniform texture2D normal_texture;
layout (set = 1, binding = 4) uniform texture2D occlusion_texture;
layout (set = 1, binding = 5) uniform texture2D emissive_texture;

layout (location = 0) in vec3 in_position_vs;
layout (location = 1) in vec3 in_normal_vs;
// w component is a sign value (-1 or +1) indicating handedness of the tangent basis
// see GLTF spec for more info
layout (location = 2) in vec3 in_tangent_vs;
layout (location = 3) in vec3 in_binormal_vs;
layout (location = 4) in vec2 in_uv;

// Force early depth testing, this is likely not strictly necessary
layout(early_fragment_tests) in;

layout (location = 0) out vec4 out_color;

vec4 normal_map(mat3 tangent_normal_binormal, texture2D t, sampler s, vec2 uv) {
    // Sample the normal and unflatten it from the texture (i.e. convert
    // range of [0, 1] to [-1, 1])
    vec3 normal = texture(sampler2D(t, s), uv).xyz;
    normal = normal * 2.0 - 1.0;

    // Transform the normal from the texture with the TNB matrix, which will put
    // it into the TNB's space (view space))
    normal = normal * tangent_normal_binormal;
    return normalize(vec4(normal, 0.0));
}

void main() {
    //TODO: Consider adding a global ambient color to per_frame_data
    // Base color

    // Sample the base color, if it exists
    vec4 base_color = material_data.base_color_factor;
    if (material_data.has_base_color_texture) {
        base_color *= texture(sampler2D(base_color_texture, smp), in_uv);
    }

    // Sample the emissive color, if it exists
    vec4 emissive_color = vec4(material_data.emissive_factor, 1);
    if (material_data.has_emissive_texture) {
        emissive_color *= texture(sampler2D(emissive_texture, smp), in_uv);
        base_color = vec4(1.0, 1.0, 0.0, 1.0);
    }

    // Calculate the normal (use the normal map if it exists)
    vec4 normal_vs;
    if (material_data.has_normal_texture) {
        mat3 tbn = mat3(in_tangent_vs, in_binormal_vs, in_normal_vs);
        normal_vs = normal_map(tbn, normal_texture, smp, in_uv);
    } else {
        normal_vs = normalize(vec4(in_normal_vs, 0));
    }


    // view space eye is 0,0,0
    //vec3 eye_position_vs = vec3(0, 0, 0);
    //vec3 eye_direction = in_position_vs - eye_position_vs;


    // just pick something
    vec3 light_vs = normalize(vec3(-5, -5, -5));

    float NdotL = max(dot(normal_vs.xyz, light_vs), 0);
    base_color = vec4(base_color.xyz * NdotL, 1);







    
    // Point Lights
    //for (uint i = 0; i < per_frame_data.point_light_count; ++i) {
    //    base_color = base_color * per_frame_data.point_lights[0].color;
    //}

    out_color = base_color;
}
