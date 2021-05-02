@echo off
setlocal
cd /D "%~dp0"

cargo run --package rafx-shader-processor -- ^
--glsl-path glsl/*.vert glsl/*.frag glsl/*.comp ^
--rs-path src ^
--metal-generated-src-path generated_msl ^
--cooked-shaders-path ../assets/shaders ^
--package-vk ^
--package-metal ^
&& cargo fmt && cargo test --package shaders