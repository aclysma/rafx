#version 450

layout (set = 0, binding = 1) uniform sampler smp;
layout (set = 0, binding = 2) uniform texture2D tex;

layout (location = 0) in vec2 in_uv;

layout (location = 0) out vec4 out_color;

void main() {
    out_color = texture(sampler2D(tex, smp), in_uv);
}
