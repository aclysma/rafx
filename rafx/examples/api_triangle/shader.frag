#version 450

layout (set = 0, binding = 0) uniform PerViewData {
    vec4 uniform_color;
} uniform_data;

layout (location = 0) in vec4 in_color;

layout (location = 0) out vec4 out_color;

void main() {
    //out_color = in_color;
    out_color = uniform_data.uniform_color;
}
