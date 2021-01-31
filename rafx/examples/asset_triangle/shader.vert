#version 450

// These "semantic" annotations are matched with vertex data, allowing pipelines to be produced as needed for whatever
// data is in use at runtime

// @[semantic("POSITION")]
layout (location = 0) in vec4 pos;
// @[semantic("COLOR")]
layout (location = 1) in vec4 in_color;

layout (location = 0) out vec4 out_color;

void main() {
    out_color = in_color;
    gl_Position = pos;
}
