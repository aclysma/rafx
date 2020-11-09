#version 450
#extension GL_ARB_separate_shader_objects : enable

// Largely taken from https://learnopengl.com/Advanced-Lighting/Bloom

layout (set = 0, binding = 0) uniform texture2D tex;

// @[immutable_samplers([
//         (
//             mag_filter: Nearest,
//             min_filter: Nearest,
//             address_mode_u: ClampToEdge,
//             address_mode_v: ClampToEdge,
//             address_mode_w: ClampToEdge,
//             anisotropy_enable: false,
//             max_anisotropy: 1.0,
//             border_color: FloatOpaqueWhite,
//             unnormalized_coordinates: false,
//             compare_enable: false,
//             compare_op: Always,
//             mipmap_mode: Linear,
//             mip_lod_bias: 0,
//             min_lod: 0,
//             max_lod: 1
//         )
// ])]
layout (set = 0, binding = 1) uniform sampler smp;

// @[export]
// @[internal_buffer]
layout (set = 0, binding = 2) uniform Config {
    bool horizontal;
} config;


layout (location = 0) in vec2 inUV;

layout (location = 0) out vec4 out_blur;

void main()
{
    float weight[5] = float[] (0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);
    vec2 tex_offset = 1.0 / textureSize(sampler2D(tex, smp), 0);
    vec3 result = texture(sampler2D(tex, smp), inUV).rgb * weight[0];

    if (config.horizontal) {
        for(int i = 1; i < 5; ++i)
        {
            result += texture(sampler2D(tex, smp), inUV + vec2(tex_offset.x * i, 0.0)).rgb * weight[i];
            result += texture(sampler2D(tex, smp), inUV - vec2(tex_offset.x * i, 0.0)).rgb * weight[i];
        }
    } else {
        for(int i = 1; i < 5; ++i)
        {
            result += texture(sampler2D(tex, smp), inUV + vec2(0.0, tex_offset.y * i)).rgb * weight[i];
            result += texture(sampler2D(tex, smp), inUV - vec2(0.0, tex_offset.y * i)).rgb * weight[i];
        }
    }

    out_blur = vec4(result, 1.0);
}
