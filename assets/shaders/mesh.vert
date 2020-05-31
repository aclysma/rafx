#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout(set = 2, binding = 0) uniform Args {
    mat4 mvp;
} uniform_buffer;

layout (location = 0) in vec3 in_pos;
layout (location = 1) in vec3 in_normal;
layout (location = 2) in vec3 in_tangent;
layout (location = 3) in vec2 in_uv;

layout (location = 0) out vec3 out_normal;
layout (location = 1) out vec3 out_tangent;
layout (location = 2) out vec2 out_uv;

void main() {
    gl_Position = uniform_buffer.mvp * vec4(in_pos, 1.0);
    out_normal = in_normal;
    out_tangent = in_tangent;
    out_uv = in_uv;
}
