
export PATH=~/dev/sdk/vulkansdk-macos-1.2.148.1/macOS/bin/glslc:$PATH

export ARGS="-O -g"

glslc $ARGS sprite.vert -o sprite.vert.spv
glslc $ARGS sprite.frag -o sprite.frag.spv

glslc $ARGS mesh.vert -o mesh.vert.spv
glslc $ARGS mesh.frag -o mesh.frag.spv

glslc $ARGS mesh_shadow_map.vert -o mesh_shadow_map.vert.spv

glslc $ARGS debug.vert -o debug.vert.spv
glslc $ARGS debug.frag -o debug.frag.spv

glslc $ARGS bloom_extract.vert -o bloom_extract.vert.spv
glslc $ARGS bloom_extract.frag -o bloom_extract.frag.spv

glslc $ARGS bloom_blur.vert -o bloom_blur.vert.spv
glslc $ARGS bloom_blur.frag -o bloom_blur.frag.spv

glslc $ARGS bloom_combine.vert -o bloom_combine.vert.spv
glslc $ARGS bloom_combine.frag -o bloom_combine.frag.spv

glslc $ARGS imgui.vert -o imgui.vert.spv
glslc $ARGS imgui.frag -o imgui.frag.spv
