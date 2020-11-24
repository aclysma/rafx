#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

// Keep this identical to PerObjectData in mesh.vert
// @[internal_buffer]
layout(set = 2, binding = 0) uniform PerObjectData {
    mat4 model;
    mat4 model_view;
    mat4 model_view_proj;
} per_object_data;

// @[semantic("POSITION")]
layout (location = 0) in vec3 in_pos;

void main() {
    gl_Position = per_object_data.model_view_proj * vec4(in_pos, 1.0);
}
