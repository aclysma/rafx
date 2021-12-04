@echo off
setlocal
cd /D "%~dp0"

cargo run --package rafx-shader-processor -- ^
--glsl-path glsl ^
--rs-mod-path src/shaders ^
--cooked-shaders-path assets/rafx-plugins/shaders ^
--package-vk ^
--package-metal && cargo fmt && cargo test --package rafx-plugins --features "legion"