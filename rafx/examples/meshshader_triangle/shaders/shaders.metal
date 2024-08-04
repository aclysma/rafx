#include <metal_stdlib>

using namespace metal;

struct VertexOut {
    float4 position [[position]];
};

using mesh_t = mesh<VertexOut, void, 3, 1, topology::triangle>;

[[mesh]] void main_ms(mesh_t m, uint thread_index [[thread_position_in_threadgroup]]) {
    VertexOut v;

    if (thread_index == 0) {
        v.position = float4(0.0, 1.0, 0.0, 1.0);
        m.set_vertex(0, v);

        m.set_index(0, 0);
        m.set_index(1, 1);
        m.set_index(2, 2);

        m.set_primitive_count(1);
    } else if (thread_index == 1) {
        v.position = float4(1.0, -1.0, 0.0, 1.0);
        m.set_vertex(1, v);
    } else if (thread_index == 2) {
        v.position = float4(-1.0, -1.0, 0.0, 1.0);
        m.set_vertex(2, v);
    }
}

fragment half4 main_ps() {
    return half4(0.1, 1.0, 0.1, 1.0);
}
