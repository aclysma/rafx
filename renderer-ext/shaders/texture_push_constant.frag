#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
#extension GL_EXT_nonuniform_qualifier : enable

layout (set = 0, binding = 0) uniform UBO{
    vec3 color;
} ubo;

layout (set = 0, binding = 1) uniform sampler smp;

layout (set = 1, binding = 0) uniform texture2D tex[100];

layout (push_constant) uniform PushConstants {
    uint textureIndex;
} pushConstants;

layout (location = 0) in vec2 o_uv;

layout (location = 0) out vec4 uFragColor;

void main() {
    //vec4 color = texture(tex[0], o_uv);
    vec4 color = texture(sampler2D(tex[pushConstants.textureIndex], smp), o_uv);
    uFragColor = color;
}
