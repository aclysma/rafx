
cargo run --package rafx-shader-processor -- \
--glsl-path glsl \
--optimize-shaders \
--rs-mod-path src/shaders \
--cooked-shaders-path assets/rafx-plugins/shaders \
--metal-generated-src-path processed_shaders/msl \
--package-vk \
--package-metal && cargo fmt && cargo test --package rafx-plugins