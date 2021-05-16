cargo run --package rafx-shader-processor -- \
--glsl-path *.vert *.frag *.comp \
--spv-path processed_shaders \
--metal-generated-src-path processed_shaders \
--gles2-generated-src-path processed_shaders \
--gles3-generated-src-path processed_shaders \
--cooked-shaders-path assets \
--package-all