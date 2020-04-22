#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (set = 0, binding = 0) uniform UBO{
    vec3 color;
} ubo;

layout (set = 0, binding = 1) uniform texture2D tex[];
layout (set = 0, binding = 2) uniform sampler smp;

layout (location = 0) in vec2 o_uv;

layout (location = 0) out vec4 uFragColor;

void main() {
    //vec4 color = texture(tex[0], o_uv);
    vec4 color = texture(sampler2D(tex[0], smp), o_uv);
    uFragColor = color;
}
