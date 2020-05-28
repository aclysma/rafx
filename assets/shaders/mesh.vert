#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout(set = 2, binding = 0) uniform Args {
    mat4 mvp;
} uniform_buffer;

layout (location = 0) in vec3 pos;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec2 uv;

layout (location = 0) out vec3 o_normal;
layout (location = 1) out vec2 o_uv;

void main() {
    o_uv = uv;
    o_normal = normal;
    gl_Position = uniform_buffer.mvp * vec4(pos, 1.0);
}
