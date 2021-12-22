#version 450
#extension GL_ARB_separate_shader_objects : enable

#include "debug_pip.glsl"

layout(location = 0) in vec2 in_texcoord;

layout(location = 0) out vec4 out_color;

void main() {
    out_color = texture(sampler2D(debug_pip_tex, smp), in_texcoord);
}
