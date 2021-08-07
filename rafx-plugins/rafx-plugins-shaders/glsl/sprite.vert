#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

#include "sprite.glsl"

// @[semantic("POSITION")]
layout (location = 0) in vec3 pos;

// @[semantic("TEXCOORD")]
layout (location = 1) in vec2 uv;

// @[semantic("COLOR")]
layout (location = 2) in vec4 color;

layout (location = 0) out vec2 o_uv;
layout (location = 1) out vec4 o_color;

void main() {
    o_uv = uv;
    o_color = color;
    gl_Position = uniform_buffer.mvp * vec4(pos, 1.0);
}
