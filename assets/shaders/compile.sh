
export PATH=~/dev/sdk/vulkansdk-macos-1.2.148.1/macOS/bin/glslc:$PATH

glslc sprite.vert -o sprite.vert.spv -g
glslc sprite.frag -o sprite.frag.spv -g

glslc mesh.vert -o mesh.vert.spv -g
glslc mesh.frag -o mesh.frag.spv -g

glslc mesh_shadow_map.vert -o mesh_shadow_map.vert.spv -g

glslc debug.vert -o debug.vert.spv -g
glslc debug.frag -o debug.frag.spv -g

glslc bloom_extract.vert -o bloom_extract.vert.spv -g
glslc bloom_extract.frag -o bloom_extract.frag.spv -g

glslc bloom_blur.vert -o bloom_blur.vert.spv -g
glslc bloom_blur.frag -o bloom_blur.frag.spv -g

glslc bloom_combine.vert -o bloom_combine.vert.spv -g
glslc bloom_combine.frag -o bloom_combine.frag.spv -g

glslc imgui.vert -o imgui.vert.spv -g
glslc imgui.frag -o imgui.frag.spv -g
