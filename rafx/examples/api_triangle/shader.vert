#version 450

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
    mat4 view[1];
    mat4 view_proj;
    vec4 ambient_light;
    int point_light_count;
    int directional_light_count;
    int spot_light_count;
    PointLight point_lights[16];
    DirectionalLight directional_lights[16];
    SpotLight spot_lights[16];
    ShadowMap2DData shadow_map_2d_data[32];
    ShadowMapCubeData shadow_map_cube_data[16];
} per_view_data;

// @[semantic("POSITION")]
layout (location = 0) in vec4 pos;

// @[semantic("COLOR")]
layout (location = 1) in vec4 in_color;

layout (location = 0) out vec4 out_color;

void main() {
    out_color = in_color;
    gl_Position = pos;
}
