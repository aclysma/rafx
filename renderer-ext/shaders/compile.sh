
export PATH=~/dev/sdk/vulkansdk-macos-1.2.131.2/macOS/bin/glslc:$PATH
glslc texture.vert -o texture.vert.spv
glslc texture_push_constant.frag -o texture_push_constant.frag.spv
glslc texture_many_sets.frag -o texture_many_sets.frag.spv

glslc imgui.vert -o imgui.vert.spv
glslc imgui.frag -o imgui.frag.spv