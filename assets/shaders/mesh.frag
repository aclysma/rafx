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
    mat4 view;
    mat4 proj;
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

layout (location = 0) in vec3 o_normal;
layout (location = 1) in vec2 o_uv;

layout (location = 0) out vec4 uFragColor;

void main() {
    // Base color
    uFragColor = vec4(1.0, 1.0, 1.0, 1.0);
    uFragColor *= uFragColor * material_data.base_color_factor;
    if (material_data.has_base_color_texture) {
        uFragColor *= texture(sampler2D(base_color_texture, smp), o_uv);
    }

    
    
    // Point Lights
    for (uint i = 0; i < per_frame_data.point_light_count; ++i) {
        uFragColor = uFragColor * per_frame_data.point_lights[0].color;
    }
}
