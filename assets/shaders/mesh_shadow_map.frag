#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

// References:
// https://www.3dgep.com/forward-plus/
// - Basic framework for forward/deferred/forward+ in non-PBR
// https://learnopengl.com/PBR/Theory
// - PBR
// https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
// https://google.github.io/filament/Filament.md.html
//
// The unreal paper is straightforward and practical for PBR. The filament one is more thorough and covers some slower
// but more accurate approaches.

const float PI = 3.14159265359;

//
// Per-Frame Pass
//
struct PointLight {
    vec3 position_ws;
    vec3 position_vs;
    vec4 color;
    float range;
    float intensity;
};

struct DirectionalLight {
    vec3 direction_ws;
    vec3 direction_vs;
    vec4 color;
    float intensity;
};

struct SpotLight {
    vec3 position_ws;
    vec3 direction_ws;
    vec3 position_vs;
    vec3 direction_vs;
    vec4 color;
    float spotlight_half_angle;
    float range;
    float intensity;
};

layout (set = 0, binding = 0) uniform PerFrameData {
    vec4 ambient_light;
    uint point_light_count;
    uint directional_light_count;
    uint spot_light_count;
    PointLight point_lights[16];
    DirectionalLight directional_lights[16];
    SpotLight spot_lights[16];
} per_frame_data;

layout (set = 0, binding = 1) uniform sampler smp;

//
// Per-Material Bindings
//
struct MaterialData {
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
};

layout (set = 1, binding = 0) uniform MaterialDataUbo {
    MaterialData data;
} material_data_ubo;

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

void main() {
    out_color = vec4(1.0);
}
