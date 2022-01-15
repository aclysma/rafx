#version 450
#extension GL_ARB_separate_shader_objects : enable

#include "util_blit_image.glsl"

layout(location = 0) in vec2 in_texcoord;

layout(location = 0) out vec4 out_color;

void main() {
    out_color = texture(sampler2D(src_tex, smp), in_texcoord);
}
