#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

// @[internal_buffer]
layout(set = 2, binding = 0) uniform PerObjectData {
    mat4 model;
    mat4 model_view;
    mat4 model_view_proj;
} per_object_data;

// We don't use any of this, but validation warnings emit if we don't include them
layout (location = 0) in vec3 in_pos;
layout (location = 1) in vec3 in_normal;
layout (location = 2) in vec4 in_tangent;
layout (location = 3) in vec2 in_uv;

void main() {
    gl_Position = per_object_data.model_view_proj * vec4(in_pos, 1.0);
}
