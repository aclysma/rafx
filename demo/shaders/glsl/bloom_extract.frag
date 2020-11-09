#version 450
#extension GL_ARB_separate_shader_objects : enable

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

layout (location = 0) in vec2 inUV;

layout (location = 0) out vec4 out_sdr;
layout (location = 1) out vec4 out_bloom;

void main()
{
    vec3 color = texture(sampler2D(tex, smp), inUV).rgb;

    // Constant from https://en.wikipedia.org/wiki/Relative_luminance
    float brightness = dot(color, vec3(0.2126, 0.7152, 0.0722));
    if (brightness > 1.0f) {
        out_bloom = vec4(color, 1.0);
    } else {
        out_bloom = vec4(0.0, 0.0, 0.0, 1.0);
    }

    out_sdr = vec4(color, 1.0);
}
