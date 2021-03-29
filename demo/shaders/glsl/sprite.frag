#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

#include "sprite.glsl"

layout (location = 0) in vec2 o_uv;
layout (location = 1) in vec4 o_color;

layout (location = 0) out vec4 uFragColor;

void main() {
    uFragColor = texture(sampler2D(tex, smp), o_uv) * o_color;
}
