//
// Per-Frame Pass
//
struct PointLight {
    vec3 position_ws;
    vec3 position_vs;
    vec4 color;
    float range;
    float intensity;

    // Index into shadow_map_images_cube and per_view_data.shadow_map_cube_data
    int shadow_map;
};

struct DirectionalLight {
    vec3 direction_ws;
    vec3 direction_vs;
    vec4 color;
    float intensity;

    // Index into shadow_map_images and per_view_data.shadow_map_2d_data
    int shadow_map;
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

    // Index into shadow_map_images and per_view_data.shadow_map_2d_data
    int shadow_map;
};

struct ShadowMap2DData {
    mat4 shadow_map_view_proj;
    vec3 shadow_map_light_dir;
};

struct ShadowMapCubeData {
    // We just need the cubemap's near/far z values, not the whole projection matrix
    float cube_map_projection_near_z;
    float cube_map_projection_far_z;
};

// @[export]
// @[internal_buffer]
layout (set = 0, binding = 0) uniform PerViewData {
    vec4 ambient_light;
    uint point_light_count;
    uint directional_light_count;
    uint spot_light_count;
    PointLight point_lights[16];
    DirectionalLight directional_lights[16];
    SpotLight spot_lights[16];
    ShadowMap2DData shadow_map_2d_data[32];
    ShadowMapCubeData shadow_map_cube_data[16];
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

// @[immutable_samplers([
//     (
//         mag_filter: Linear,
//         min_filter: Linear,
//         mip_map_mode: Linear,
//         address_mode_u: ClampToBorder,
//         address_mode_v: ClampToBorder,
//         address_mode_w: ClampToBorder,
//         anisotropy_enable: true,
//         max_anisotropy: 16.0,
//         compare_op: Greater,
//     )
// ])]
layout (set = 0, binding = 2) uniform sampler smp_depth;

// @[export]
layout (set = 0, binding = 3) uniform texture2D shadow_map_images[32];

// @[export]
layout (set = 0, binding = 4) uniform textureCube shadow_map_images_cube[16];

//
// Per-Material Bindings
//
struct MaterialData {
    vec4 base_color_factor;
    vec3 emissive_factor;
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

// @[export]
// @[internal_buffer]
// @[slot_name("per_material_data")]
layout (set = 1, binding = 0) uniform MaterialDataUbo {
    MaterialData data;
} per_material_data;

// @[export]
// @[slot_name("base_color_texture")]
layout (set = 1, binding = 1) uniform texture2D base_color_texture;

// @[export]
// @[slot_name("metallic_roughness_texture")]
layout (set = 1, binding = 2) uniform texture2D metallic_roughness_texture;

// @[export]
// @[slot_name("normal_texture")]
layout (set = 1, binding = 3) uniform texture2D normal_texture;

// @[export]
// @[slot_name("occlusion_texture")]
layout (set = 1, binding = 4) uniform texture2D occlusion_texture;

// @[export]
// @[slot_name("emissive_texture")]
layout (set = 1, binding = 5) uniform texture2D emissive_texture;

// @[export]
// @[internal_buffer]
layout(set = 2, binding = 0) uniform PerObjectData {
    mat4 model;
    mat4 model_view;
    mat4 model_view_proj;
} per_object_data;
