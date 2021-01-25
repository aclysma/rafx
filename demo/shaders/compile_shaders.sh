
cargo run --package rafx-shader-processor -- --glsl-path glsl/*.vert glsl/*.frag glsl/*.comp --rs-path src --metal-generated-src-path generated_msl --cooked-shaders-path ../assets/shaders && cargo fmt && cargo test --package shaders
#cargo run --package rafx-shader-processor -- --trace --glsl-path glsl/*.vert glsl/*.frag glsl/*.comp --rs-path src --cooked-shaders-path ../assets/shaders
#cargo test --package shaders
