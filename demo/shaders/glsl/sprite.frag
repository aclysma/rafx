#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

// @[immutable_samplers([
//         (
//             mag_filter: Linear,
//             min_filter: Linear,
//             mip_map_mode: Linear,
//             address_mode_u: Repeat,
//             address_mode_v: Repeat,
//             address_mode_w: Repeat,
//         )
// ])]
layout (set = 0, binding = 1) uniform sampler smp;

// @[export]
layout (set = 1, binding = 0) uniform texture2D tex;

layout (location = 0) in vec2 o_uv;

layout (location = 0) out vec4 uFragColor;

void main() {
    //vec4 color = texture(tex[0], o_uv);
    vec4 color = texture(sampler2D(tex, smp), o_uv);
    uFragColor = color;
}
