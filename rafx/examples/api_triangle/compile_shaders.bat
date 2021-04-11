@echo off
setlocal
cd /D "%~dp0"

cargo run --package rafx-shader-processor -- --glsl-path *.vert *.frag *.comp --spv-path processed_shaders --metal-generated-src-path processed_shaders --gl-generated-src-path processed_shaders --gles-generated-src-path processed_shaders
