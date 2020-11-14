
export PATH=~/dev/sdk/vulkansdk-macos-1.2.148.1/macOS/bin/glslc:$PATH

export ARGS="-O -g"

glslc $ARGS test.frag -o test.spv
