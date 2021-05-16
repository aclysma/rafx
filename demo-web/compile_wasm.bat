@echo off
setlocal
cd /D "%~dp0"

wasm-pack build --target web --out-name web --out-dir pkg
