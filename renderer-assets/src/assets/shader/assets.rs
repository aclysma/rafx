use serde::{Deserialize, Serialize};
use type_uuid::*;
use crate::vk_description as dsc;

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
#[uuid = "e0ae2222-1a44-4022-af95-03c9101ac89e"]
pub struct ShaderAsset {
    pub shader: dsc::ShaderModule,
}
