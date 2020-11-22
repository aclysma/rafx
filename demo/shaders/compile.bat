@echo off
setlocal
cd /D "%~dp0"

cargo run --package shader-processor -- --trace --glsl-path glsl/*.vert glsl/*.frag --rs-path src --cooked-shaders-path ../../assets/shaders && cargo fmt && cargo test --package shaders