#version 450
#extension GL_ARB_separate_shader_objects : enable

#include "egui.glsl"

layout(location = 0) in vec2 uv;
layout(location = 1) in vec4 color;

layout(location = 0) out vec4 out_color;

void main() {
    // This sample should probably be converted sRGB -> linear (some GPUs don't support R8_SRGB fully, so it's sRGB data
    // in a linear texture). But the text looks much better if we don't do the conversion.
    out_color = texture(sampler2D(tex, smp), uv).rrrr * color;
}