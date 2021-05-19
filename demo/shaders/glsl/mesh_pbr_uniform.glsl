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

const int MAX_POINT_LIGHTS = 16;
const int MAX_DIRECTIONAL_LIGHTS = 16;
const int MAX_SPOT_LIGHTS = 16;

const int MAX_SHADOW_MAPS_2D = MAX_DIRECTIONAL_LIGHTS + MAX_SPOT_LIGHTS;
const int SHADOW_MAP_2D_ARRAY_LEN = 32; // TODO(dvd): Shader processor can't handle const math in array indices.

const int MAX_SHADOW_MAPS_CUBE = MAX_POINT_LIGHTS;
const int SHADOW_MAP_CUBE_ARRAY_LEN = MAX_POINT_LIGHTS;

// @[export]
// @[internal_buffer]
layout (set = 0, binding = 0) uniform PerViewData {
    mat4 view;
    mat4 view_proj;
    vec4 ambient_light;
    uint point_light_count;
    uint directional_light_count;
    uint spot_light_count;
    PointLight point_lights[MAX_POINT_LIGHTS];
    DirectionalLight directional_lights[MAX_DIRECTIONAL_LIGHTS];
    SpotLight spot_lights[MAX_SPOT_LIGHTS];
    ShadowMap2DData shadow_map_2d_data[SHADOW_MAP_2D_ARRAY_LEN];
    ShadowMapCubeData shadow_map_cube_data[SHADOW_MAP_CUBE_ARRAY_LEN];
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
//         address_mode_u: ClampToEdge,
//         address_mode_v: ClampToEdge,
//         address_mode_w: ClampToEdge,
//         anisotropy_enable: true,
//         max_anisotropy: 16.0,
//         compare_op: Greater,
//     )
// ])]
layout (set = 0, binding = 2) uniform sampler smp_depth;

// @[export]
layout (set = 0, binding = 3) uniform texture2D shadow_map_images[SHADOW_MAP_2D_ARRAY_LEN];

// @[export]
layout (set = 0, binding = 4) uniform textureCube shadow_map_images_cube[SHADOW_MAP_CUBE_ARRAY_LEN];

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
// @[internal_buffer]
layout(set = 2, binding = 0) uniform PerObjectData {
    mat4 model;
} per_object_data;