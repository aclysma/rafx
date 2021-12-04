cargo run --package rafx-shader-processor -- \
--glsl-path . \
--spv-path processed_shaders \
--metal-generated-src-path processed_shaders \
--gles2-generated-src-path processed_shaders \
--gles3-generated-src-path processed_shaders \
--cooked-shaders-path cooked_shaders \
--package-all