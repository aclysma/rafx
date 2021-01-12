xcrun -sdk macosx metal -c shaders.metal -o shaders.air
xcrun -sdk macosx metallib shaders.air -o shaders.metallib
rm shaders.air