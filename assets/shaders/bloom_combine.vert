
#version 450

// This shader takes no input except for gl_VertexIndex. It is intended to be drawn with three elements. This will
// produce a single triangle that covers the screen.
// https://www.saschawillems.de/blog/2016/08/13/vulkan-tutorial-on-rendering-a-fullscreen-quad-without-buffers/

layout (location = 0) out vec2 outUV;

out gl_PerVertex
{
    vec4 gl_Position;
};

void main()
{
    outUV = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);
    gl_Position = vec4(outUV * 2.0f - 1.0f, 0.0f, 1.0f);
}