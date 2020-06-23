#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(set = 0, binding = 0) uniform Args {
    mat4 mvp;
} uniform_buffer;

layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 in_uv;
layout(location = 2) in vec4 in_color;

layout(location = 0) out vec2 uv;
layout(location = 1) out vec4 color;

void main() {
    uv = in_uv;
    color = in_color;
    gl_Position = uniform_buffer.mvp * vec4(pos.x, pos.y, 0.0, 1.0);
}