//
// Per-Frame Pass
//
#include "lights_processing.glsl"

struct PointLight {
    vec3 position_ws;
    float range;
    vec3 position_vs;
    float intensity;
    vec4 color;

    // Index into shadow_map_images_cube and per_view_data.shadow_map_cube_data
    int shadow_map;
};

struct DirectionalLight {
    vec3 direction_ws;
    float intensity;
    vec4 color;

    vec3 direction_vs;
    // Index into shadow_map_images and per_view_data.shadow_map_2d_data
    int shadow_map;
};

struct SpotLight {
    vec3 position_ws;
    float range;
    vec3 position_vs;
    float intensity;
    vec4 color;

    //spotlight only
    vec3 direction_ws;
    float spotlight_half_angle;
    vec3 direction_vs;

    // Index into shadow_map_images and per_view_data.shadow_map_2d_data
    int shadow_map;
};

struct ShadowMap2DData {
    vec2 uv_min;
    vec2 uv_max;
    mat4 shadow_map_view_proj;
    vec3 shadow_map_light_dir;
};

struct ShadowMapCubeData {
    // We just need the cubemap's near/far z values, not the whole projection matrix
    // xy=min, zw=max
    vec4 uv_min_uv_max[6];
    float cube_map_projection_near_z;
    float cube_map_projection_far_z;
};

// @[export]
// @[internal_buffer]
layout (set = 0, binding = 0) uniform PerViewData {
    mat4 view;
    mat4 view_proj;
    vec4 ambient_light;
    vec2 jitter_amount;
    uint viewport_width;
    uint viewport_height;
    float mip_bias;
    float ndf_filter_amount;
    uint directional_light_count;
    bool use_clustered_lighting;
    DirectionalLight directional_lights[8];
    ShadowMap2DData shadow_map_2d_data[96];
    ShadowMapCubeData shadow_map_cube_data[32];
} per_view_data;

// @[immutable_samplers([
//     (
//         mag_filter: Linear,
//         min_filter: Linear,
//         mip_map_mode: Linear,
//         address_mode_u: Repeat,
//         address_mode_v: Repeat,
//         address_mode_w: Repeat,
//         max_anisotropy: 16.0,
//     )
// ])]
layout (set = 0, binding = 1) uniform sampler smp;

// Don't use anisotropy because light clustering can cause divergent execution
// @[immutable_samplers([
//     (
//         mag_filter: Linear,
//         min_filter: Linear,
//         mip_map_mode: Linear,
//         address_mode_u: ClampToEdge,
//         address_mode_v: ClampToEdge,
//         address_mode_w: ClampToEdge,
//         max_anisotropy: 1.0,
//         compare_op: Greater,
//     )
// ])]
layout (set = 0, binding = 2) uniform sampler smp_depth_linear;

// Can't use linear PCF with cubemaps,
// Don't use anisotropy because light clustering can cause divergent execution
// @[immutable_samplers([
//     (
//         mag_filter: Nearest,
//         min_filter: Nearest,
//         mip_map_mode: Nearest,
//         address_mode_u: ClampToEdge,
//         address_mode_v: ClampToEdge,
//         address_mode_w: ClampToEdge,
//         max_anisotropy: 1.0,
//         compare_op: Greater,
//     )
// ])]
layout (set = 0, binding = 3) uniform sampler smp_depth_nearest;

// @[export]
layout (set = 0, binding = 4) uniform texture2D shadow_map_atlas;

// @[export]
layout (set = 0, binding = 5) buffer LightBinOutput {
    LightBinningOutput data;
} light_bin_output;

struct LightInList {
    vec3 position_ws;
    float range;
    vec3 position_vs;
    float intensity;
    vec4 color;

    //spotlight only
    vec3 spotlight_direction_ws;
    float spotlight_half_angle;
    vec3 spotlight_direction_vs;

    // Index into shadow_map_images and per_view_data.shadow_map_2d_data
    int shadow_map;
};

// @[export]
layout (set = 0, binding = 6) buffer AllLights {
    uint light_count;
    LightInList data[512];
} all_lights;

//
// Per-Material Bindings
//
struct MaterialData {
    vec4 base_color_factor;
    vec3 emissive_factor;
    float metallic_factor;
    float roughness_factor;
    float normal_texture_scale;
    float alpha_threshold;
    bool enable_alpha_blend;
    bool enable_alpha_clip;
    bool has_base_color_texture;
    bool base_color_texture_has_alpha_channel;
    bool has_metallic_roughness_texture;
    bool has_normal_texture;
    bool has_emissive_texture;
};

// @[export]
// @[internal_buffer]
// @[slot_name("per_material_data")]
layout (set = 2, binding = 0) uniform MaterialDataUbo {
    MaterialData data;
} per_material_data;
