
// Keep this identical to PerObjectData in mesh.vert
// @[export]
// @[internal_buffer]
layout(set = 2, binding = 0) uniform PerObjectData {
    mat4 model;
    mat4 model_view;
    mat4 model_view_proj;
} per_object_data;
