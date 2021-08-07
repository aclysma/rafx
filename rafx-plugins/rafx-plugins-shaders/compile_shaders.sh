
cargo run --package rafx-shader-processor -- \
--glsl-path glsl/*.vert glsl/*.frag glsl/*.comp \
--rs-path src \
--cooked-shaders-path ../assets/rafx-plugins/shaders \
--package-vk \
--package-metal \
&& cargo fmt && cargo test --package rafx-plugins-shaders
