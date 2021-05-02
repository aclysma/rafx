#version 450

layout (set = 0, binding = 0) uniform PerViewData {
    mat4 mvp;
} uniform_data;

layout (location = 0) in vec2 pos;
layout (location = 1) in vec2 in_uv;

layout (location = 0) out vec2 out_uv;

void main() {
    out_uv = in_uv;
    gl_Position = vec4(pos, 0.0, 1.0);
}
