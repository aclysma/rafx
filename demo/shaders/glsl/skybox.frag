#version 450
#extension GL_ARB_separate_shader_objects : enable

#include "skybox.glsl"

layout(location = 0) in vec3 in_texcoord;

layout(location = 0) out vec4 out_color;

void main() {
    out_color = texture(samplerCube(skybox_tex, smp), in_texcoord);
}
