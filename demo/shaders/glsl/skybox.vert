#version 450
#extension GL_ARB_separate_shader_objects : enable

#include "skybox.glsl"

layout(location = 0) out vec3 out_texcoord;

void main() {
    // Generate a triangle that covers the whole screen. This shader should be draw as 3 vertices
    gl_Position = vec4(((gl_VertexIndex << 1) & 2) * 2.0 - 1.0, (gl_VertexIndex & 2) * 2.0 - 1.0, 0.0, 1.0);
    out_texcoord = mat3(uniform_buffer.inverse_view) * (uniform_buffer.inverse_projection * gl_Position).xyz;
}
