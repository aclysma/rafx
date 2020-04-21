
export PATH=~/dev/sdk/vulkansdk-macos-1.2.131.2/macOS/bin/glslc:$PATH
glslc texture.vert -o texture.vert.spv
glslc texture.frag -o texture.frag.spv

glslc imgui.vert -o imgui.vert.spv
glslc imgui.frag -o imgui.frag.spv