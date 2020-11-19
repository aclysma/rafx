
cargo run --package shader-processor -- --trace --glsl-path glsl/*.vert glsl/*.frag --rs-path src --cooked-shaders-path ../../assets/shaders

#cargo run --package shader-processor -- --glsl_path glsl/baseline.frag
#cargo run --package shader-processor -- --glsl_path glsl/repro.frag