@echo off
setlocal
cd /D "%~dp0"

cargo run --package rafx-shader-processor -- ^
--glsl-path glsl ^
--gles2-generated-src-path processed_shaders ^
--package-all
