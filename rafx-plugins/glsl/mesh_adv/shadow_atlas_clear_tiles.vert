#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

// @[semantic("POSITION")]
layout (location = 0) in vec2 in_pos;

out float gl_ClipDistance[4];

void main() {
    // Positions are specified as UV coords, so convert to clip space coords
    vec2 clip_space = in_pos * 2.0 - 1.0;
    gl_Position = vec4(clip_space.x, -clip_space.y, 0.0, 1.0);
}
