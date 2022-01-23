#version 450

#include "../util/color.glsl"

// @[export]
layout (set = 0, binding = 0) uniform texture2D depth_tex;
// @[export]
layout (set = 0, binding = 1) uniform texture2D noise_tex;

const int SAMPLE_COUNT = 16;
const float SSAO_SAMPLE_RADIUS = 0.4;
const float SSAO_DEPTH_DISCONTINUITY_REJECT_DISTANCE = 0.1;

// @[export]
// @[internal_buffer]
layout (set = 0, binding = 2) uniform Config {
    mat4 proj;
    mat4 proj_inv;
    vec4 samples[16]; // SAMPLE_COUNT, Use Vec4 instead of Vec3 to avoid alignment problems
    vec2 random_noise_offset;
    uint frame_index;
} config;

// @[immutable_samplers([
//     (
//         mag_filter: Nearest,
//         min_filter: Nearest,
//         mip_map_mode: Nearest,
//         address_mode_u: ClampToEdge,
//         address_mode_v: ClampToEdge,
//         address_mode_w: ClampToEdge,
//     )
// ])]
layout (set = 0, binding = 3) uniform sampler smp_nearest;

// @[immutable_samplers([
//     (
//         mag_filter: Linear,
//         min_filter: Linear,
//         mip_map_mode: Nearest,
//         address_mode_u: ClampToEdge,
//         address_mode_v: ClampToEdge,
//         address_mode_w: ClampToEdge,
//     )
// ])]
layout (set = 0, binding = 4) uniform sampler smp_linear;

layout (location = 0) in vec2 inUV;

layout (location = 0) out vec4 out_image;

// https://therealmjp.github.io/posts/position-from-depth-3/
// A = FarClipDistance / (FarClipDistance - NearClipDistance)
// B = (-FarClipDistance * NearClipDistance) / (FarClipDistance - NearClipDistance)
// linear depth = ProjectionB / (depth - ProjectionA)
float depth_vs(float depth) {
    return -1.0 * (config.proj[3][2] / (depth - config.proj[2][2]));
}

vec3 pos_vs(vec2 uv, float depth) {
    vec4 pos_cs = vec4((uv * 2.0 - 1.0) * vec2(1, -1), depth, 1.0);
    vec4 pos_vs = config.proj_inv * pos_cs;
    vec3 result = pos_vs.xyz / pos_vs.w;
    return result;
}

// Normals reconstructed from depth:
// - https://wickedengine.net/2019/09/22/improved-normal-reconstruction-from-depth/
// - https://atyuwen.github.io/posts/normal-reconstruction/
// SSAO:
// - https://learnopengl.com/Advanced-Lighting/SSAO
// - This implementation is very primitive. There are better techniques now that generally raymarch to find the
//   "bent normal"/cone that light can hit the surface from. See "Practical Real-Time Strategies for Accurate Indirect Occlusion"
//   https://www.activision.com/cdn/research/Practical_Real_Time_Strategies_for_Accurate_Indirect_Occlusion_NEW%20VERSION_COLOR.pdf
//   which in addition to describing modern improvements, also has a nice summary of previous work

void main() {
    vec2 depth_texture_size = textureSize(sampler2D(depth_tex, smp_nearest), 0);
    vec2 depth_texel_size = 1.0 / depth_texture_size;

    vec2 noise_texture_size = textureSize(sampler2D(noise_tex, smp_nearest), 0);
    vec2 noise_texel_size = 1.0 / noise_texture_size;

    uvec2 pixel = uvec2(inUV * depth_texture_size);
    uvec2 noise_pixel = (pixel + uvec2(noise_texture_size * config.random_noise_offset)) % uvec2(noise_texture_size);

    vec3 noise_value = texture(sampler2D(noise_tex, smp_nearest), vec2(noise_pixel) * noise_texel_size).rgb;
    noise_value = noise_value * 2.0 - 1.0;

    float d = texture(sampler2D(depth_tex, smp_nearest), inUV).r;
    float d_linear_depth = depth_vs(d);

    vec4 taps;
    vec4 taps_linear_depth;
    vec2 extrapolated_d_error;
    vec3 P0 = pos_vs(inUV, d);
    vec3 x_dir;
    vec3 y_dir;

    taps.x = texture(sampler2D(depth_tex, smp_nearest), inUV + vec2(depth_texel_size.x * -2.0, 0)).r;
    taps.y = texture(sampler2D(depth_tex, smp_nearest), inUV + vec2(depth_texel_size.x * -1.0, 0)).r;
    taps.z = texture(sampler2D(depth_tex, smp_nearest), inUV + vec2(depth_texel_size.x * 2.0, 0)).r;
    taps.w = texture(sampler2D(depth_tex, smp_nearest), inUV + vec2(depth_texel_size.x * 1.0, 0)).r;

    taps_linear_depth.x = depth_vs(taps.x);
    taps_linear_depth.y = depth_vs(taps.y);
    taps_linear_depth.z = depth_vs(taps.z);
    taps_linear_depth.w = depth_vs(taps.w);

    extrapolated_d_error = abs(d_linear_depth - (taps_linear_depth.yw + (taps_linear_depth.yw - taps_linear_depth.xz)));
    //extrapolated_d_error = (taps.yw * taps.xz) / (2 * taps.yw - taps.xz);
    if (/*inUV.x < 0.5 * depth_texel_size.x ||*/ extrapolated_d_error.x > extrapolated_d_error.y) {
        // use +x direction
        vec3 P1 = pos_vs(inUV + vec2(depth_texel_size.x, 0.0), taps.w);
        x_dir = P1 - P0;
    } else {
        // use -x direction
        vec3 P1 = pos_vs(inUV - vec2(depth_texel_size.x, 0.0), taps.y);
        x_dir = P0 - P1;
    }

    taps.x = texture(sampler2D(depth_tex, smp_nearest), inUV + vec2(0, depth_texel_size.y * -2.0)).r;
    taps.y = texture(sampler2D(depth_tex, smp_nearest), inUV + vec2(0, depth_texel_size.y * -1.0)).r;
    taps.z = texture(sampler2D(depth_tex, smp_nearest), inUV + vec2(0, depth_texel_size.y * 2.0)).r;
    taps.w = texture(sampler2D(depth_tex, smp_nearest), inUV + vec2(0, depth_texel_size.y * 1.0)).r;

    taps_linear_depth.x = depth_vs(taps.x);
    taps_linear_depth.y = depth_vs(taps.y);
    taps_linear_depth.z = depth_vs(taps.z);
    taps_linear_depth.w = depth_vs(taps.w);

    extrapolated_d_error = abs(d_linear_depth - (taps_linear_depth.yw + (taps_linear_depth.yw - taps_linear_depth.xz)));
    //extrapolated_d_error = (taps.yw * taps.xz) / (2 * taps.yw - taps.xz);
    if (/*inUV.y < 0.5 * depth_texel_size.y ||*/ extrapolated_d_error.x > extrapolated_d_error.y) {
        // use +y direction
        vec3 P1 = pos_vs(inUV + vec2(0.0, depth_texel_size.y), taps.w);
        y_dir = P1 - P0;
    } else {
        // use -y direction
        vec3 P1 = pos_vs(inUV - vec2(0.0, depth_texel_size.y), taps.y);
        y_dir = P0 - P1;
    }

    vec3 normal = normalize(cross(x_dir, y_dir));

    // Not actually sure why this is needed. It is true that we don't know winding order from the depth buffer, so I'm
    // not sure we can truly know if we have the correct normal or not, but this seems to work, probably because
    // surfaces that actually render should have a positive z in viewspace.
    normal = -normal;
    //out_image = vec4(normal, 1.0);
    //return;

    // Generate a random 3x3 rotation matrix
    vec3 tangent = normalize(noise_value - normal * dot(noise_value, normal));
    vec3 bitangent = cross(normal, tangent);
    mat3 TBN = mat3(tangent, bitangent, normal);

    float occlusion = 0.0;
    for (int i = 0; i < SAMPLE_COUNT; ++i) {
        //Rotate the sample and find viewspace position
        vec3 sample_pos_vs = TBN * config.samples[i].xyz;
        sample_pos_vs = P0 + sample_pos_vs * SSAO_SAMPLE_RADIUS;

        // Project sample view space to clip
        vec4 sample_pos_cs = vec4(sample_pos_vs, 1.0);
        sample_pos_cs = config.proj * sample_pos_cs;
        sample_pos_cs.xyz /= sample_pos_cs.w;
        sample_pos_cs.y *= -1.0;
        sample_pos_cs.xyz = sample_pos_cs.xyz * 0.5 + 0.5;

        // Depth at sample clip position
        float sample_depth_vs = depth_vs(texture(sampler2D(depth_tex, smp_linear), sample_pos_cs.xy).r);

        // This allows us to avoid applying occlusion where there is a depth discontinuity
        float range_adjust = smoothstep(0.0, 1.0, SSAO_DEPTH_DISCONTINUITY_REJECT_DISTANCE / abs(sample_depth_vs - P0.z));

        // These are viewspace z-values, so the further away, the more negative the value will be
        // If the sample position is "under" the actual depth value, we have more occlusion
        if (sample_pos_vs.z < sample_depth_vs) {
            occlusion += range_adjust;
        }
    }

    occlusion /= float(SAMPLE_COUNT);
    vec3 ambient_factor = vec3(1.0 - occlusion);

    out_image = vec4(ambient_factor/* * ambient_factor * ambient_factor * ambient_factor*/, 1.0);
}
