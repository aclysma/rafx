#define MAX_MESHLET_SIZE 128
#define GROUP_SIZE MAX_MESHLET_SIZE
#define ROOT_SIG ""

struct VertexOut
{
    float4 PositionVS   : SV_Position;
};

[RootSignature(ROOT_SIG)]
[NumThreads(GROUP_SIZE, 1, 1)]
[OutputTopology("triangle")]
void main_ms(
    uint gtid : SV_GroupThreadID,
    uint gid : SV_GroupID,
    out indices uint3 tris[MAX_MESHLET_SIZE],
    out vertices VertexOut verts[MAX_MESHLET_SIZE]
)
{
    verts[gtid].PositionVS = float4(0.0f, 0.0f, 0.0f, 1.0f);

    int vertex_count = 3;
    int primitive_count = 1;
    SetMeshOutputCounts(vertex_count, primitive_count);
    if (gtid == 0)
    {
        tris[gtid] = uint3(0, 1, 2);
    }

    if (gtid < vertex_count)
    {
        if (gtid == 0)
        {
            verts[gtid].PositionVS = float4(-1.0, -1.0, 0.0, 1.0f);
        }
        else if (gtid == 1)
        {
            verts[gtid].PositionVS = float4(0.0, 1.0, 0.0, 1.0f);
        }
        else if (gtid == 2)
        {
            verts[gtid].PositionVS = float4(1.0, -1.0, 0.0, 1.0f);
        }
    }
}


float4 main_ps(VertexOut input) : SV_TARGET
{
    return float4(0.1, 1.0, 0.1, 1.0);
}