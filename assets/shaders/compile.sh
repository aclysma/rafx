
export PATH=~/dev/sdk/vulkansdk-macos-1.2.131.2/macOS/bin/glslc:$PATH

glslc sprite.vert -o sprite.vert.spv
glslc sprite.frag -o sprite.frag.spv

glslc imgui.vert -o imgui.vert.spv
glslc imgui.frag -o imgui.frag.spv

glslc mesh.vert -o mesh.vert.spv
glslc mesh.frag -o mesh.frag.spv

glslc debug.vert -o debug.vert.spv
glslc debug.frag -o debug.frag.spv

glslc composite.vert -o composite.vert.spv
glslc composite.frag -o composite.frag.spv