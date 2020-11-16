
cargo run --package shader-processor -- --trace --glsl_path glsl/*.vert glsl/*.frag --spv_path ../../assets/shaders --rs_path src

#cargo run --package shader-processor -- --glsl_path glsl/baseline.frag
#cargo run --package shader-processor -- --glsl_path glsl/repro.frag