
struct LightBitfieldsData {
    uint light_count[3072]; // 1 per cluster (8*16*24 clusters)
    uint bitfields[49152]; // (512 lights * (8*16*24=3072) clusters) / 32 bits in a uint)
};

struct ClusterMeta {
    uint count;
    uint first_light;
};

struct LightBinningOutput {
    uint data_write_ptr;
    uint pad0;
    uint pad1;
    uint pad2;
    ClusterMeta offsets[3072]; // 1 per cluster
    uint data[786432]; // 3072 clusters * 256 lights per cluster
};
