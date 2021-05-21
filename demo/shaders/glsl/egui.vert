#version 450
#extension GL_ARB_separate_shader_objects : enable

#include "egui.glsl"

// @[semantic("POSITION")]
layout(location = 0) in vec2 pos;

// @[semantic("TEXCOORD")]
layout(location = 1) in vec2 in_uv;

// @[semantic("COLOR")]
layout(location = 2) in vec4 in_color;

layout(location = 0) out vec2 uv;
layout(location = 1) out vec4 color;

vec3 srgb_to_linear(vec3 srgb) {
    bvec3 cutoff = lessThan(srgb, vec3(0.04045));
    vec3 higher = pow((srgb + vec3(0.055))/vec3(1.055), vec3(2.4));
    vec3 lower = srgb/vec3(12.92);

    return mix(higher, lower, cutoff);
}

void main() {
    uv = in_uv;
    color = vec4(srgb_to_linear(vec3(in_color)), in_color.a);
    gl_Position = uniform_buffer.mvp * vec4(pos.x, pos.y, 0.0, 1.0);
}