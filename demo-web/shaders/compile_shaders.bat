@echo off
setlocal
cd /D "%~dp0"

cargo run --package rafx-shader-processor -- --glsl-path *.vert *.frag --gles-generated-src-path .
