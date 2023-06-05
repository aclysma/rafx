#version 450

// @[semantic("POSITION")]
layout (location = 0) in vec4 pos;

// @[semantic("COLOR")]
layout (location = 1) in vec4 in_color;

layout (location = 0) out vec4 out_color;

void main() {
    out_color = in_color;
    gl_Position = pos;
}
