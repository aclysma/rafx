#version 450

layout (set = 0, binding = 0) uniform UniformData {
    vec4 uniform_color;
} uniform_data;


layout (push_constant) uniform PushConstantData {
    vec4 uniform_color;
} pc_data;

layout (location = 0) in vec4 in_color;

layout (location = 0) out vec4 out_color;

void main() {
    //out_color = in_color;
    //out_color = uniform_data.uniform_color;
    out_color = pc_data.uniform_color;
}
