use rafx_framework::BufferResource;
use rafx_framework::ResourceArc;
use serde::{Deserialize, Serialize};
use type_uuid::*;

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "2d6653ce-5f77-40a2-b050-f2d148699d78"]
pub struct BufferAssetData {
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

#[derive(TypeUuid, Clone)]
#[uuid = "fc3b1eb8-c986-449e-a165-6a8f4582e6c5"]
pub struct BufferAsset {
    pub buffer: ResourceArc<BufferResource>,
}
