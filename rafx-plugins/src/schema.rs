use crate::assets::mesh_adv::{MeshAdvBlendMethod, MeshAdvShadowMethod};
use hydrate_data::*;
use hydrate_model::{DataContainer, DataContainerMut, DataSetResult};
use rafx::assets::schema::*;

include!("schema_codegen.rs");

impl Into<MeshAdvBlendMethod> for MeshAdvBlendMethodEnum {
    fn into(self) -> MeshAdvBlendMethod {
        match self {
            MeshAdvBlendMethodEnum::Opaque => MeshAdvBlendMethod::Opaque,
            MeshAdvBlendMethodEnum::AlphaClip => MeshAdvBlendMethod::AlphaClip,
            MeshAdvBlendMethodEnum::AlphaBlend => MeshAdvBlendMethod::AlphaBlend,
        }
    }
}

impl Into<MeshAdvShadowMethod> for MeshAdvShadowMethodEnum {
    fn into(self) -> MeshAdvShadowMethod {
        match self {
            MeshAdvShadowMethodEnum::None => MeshAdvShadowMethod::None,
            MeshAdvShadowMethodEnum::Opaque => MeshAdvShadowMethod::Opaque,
        }
    }
}