cargo run --package rafx-shader-processor -- \
--glsl-path glsl \
--rs-mod-path src/shaders \
--cooked-shaders-path cooked_shaders \
--package-vk \
--package-metal \
--for-rafx-framework-crate && cargo fmt && cargo test --package rafx-framework
