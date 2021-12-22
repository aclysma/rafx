#version 450
#extension GL_ARB_separate_shader_objects : enable

#include "debug_pip.glsl"

layout(location = 0) out vec2 out_texcoord;

void main() {
    // Generate a triangle that covers the whole screen. This shader should be draw as 3 vertices
    vec2 coord = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);
    gl_Position = vec4(coord * 2.0 - 1.0, 0.0, 1.0);
    out_texcoord = vec2(coord.x, 1 - coord.y);
}
