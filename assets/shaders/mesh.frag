#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (set = 1, binding = 0) uniform UBO {
    vec4 base_color_factor;
    vec3 emissive_factor;
    float metallic_factor;
    float roughness_factor;
    float normal_texture_scale;
    float occlusion_texture_strength;
    float alpha_cutoff;
} ubo;

layout (set = 0, binding = 0) uniform sampler smp;

layout (set = 1, binding = 1) uniform texture2D base_color_texture;

layout (location = 0) in vec3 o_normal;
layout (location = 1) in vec2 o_uv;

layout (location = 0) out vec4 uFragColor;

void main() {
    //vec4 color = texture(tex[0], o_uv);
    //vec4 color = texture(sampler2D(tex, smp), o_uv);
    //uFragColor = color;
    //uFragColor = vec4(1.0, 1.0, 1.0, 1.0);

    uFragColor = texture(sampler2D(base_color_texture, smp), o_uv);
}
