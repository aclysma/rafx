@echo off
setlocal
cd /D "%~dp0"

cargo run --package rafx-shader-processor -- ^
--glsl-path glsl ^
--spv-path processed_shaders ^
--dx12-generated-src-path processed_shaders ^
--metal-generated-src-path processed_shaders ^
--gles2-generated-src-path processed_shaders ^
--gles3-generated-src-path processed_shaders