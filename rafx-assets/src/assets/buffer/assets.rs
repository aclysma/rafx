use crate::push_buffer::PushBuffer;
use rafx_api::RafxResourceType;
use rafx_framework::BufferResource;
use rafx_framework::ResourceArc;
use serde::{Deserialize, Serialize};
use type_uuid::*;

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "2d6653ce-5f77-40a2-b050-f2d148699d78"]
pub struct BufferAssetData {
    pub resource_type: RafxResourceType,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

impl BufferAssetData {
    pub fn from_vec<T: 'static>(
        resource_type: RafxResourceType,
        data: &Vec<T>,
    ) -> Self {
        let push_buffer = PushBuffer::from_vec(data);
        BufferAssetData {
            resource_type,
            data: push_buffer.into_data(),
        }
    }
}

#[derive(TypeUuid, Clone)]
#[uuid = "fc3b1eb8-c986-449e-a165-6a8f4582e6c5"]
pub struct BufferAsset {
    pub buffer: ResourceArc<BufferResource>,
}
