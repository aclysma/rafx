
cargo run --package rafx-shader-processor -- --glsl-path glsl/*.vert glsl/*.frag --rs-path src --cooked-shaders-path ../../assets/shaders && cargo fmt && cargo test --package shaders
#cargo run --package rafx-shader-processor -- --trace --glsl-path glsl/*.vert glsl/*.frag --rs-path src --cooked-shaders-path ../../assets/shaders
#cargo test --package shaders


#cargo run --package rafx-shader-processor -- --glsl_path glsl/baseline.frag
#cargo run --package rafx-shader-processor -- --glsl_path glsl/repro.frag