#![allow(dead_code)]

#[cfg(all(not(feature = "basic-pipeline"), feature = "legion"))]
use crate::assets::mesh_adv::{MeshAdvBlendMethod, MeshAdvShadowMethod};
use hydrate_data::*;
use rafx::assets::schema::*;
use std::cell::RefCell;
use std::rc::Rc;

include!("schema_codegen.rs");

#[cfg(all(not(feature = "basic-pipeline"), feature = "legion"))]
impl Into<MeshAdvBlendMethod> for MeshAdvBlendMethodEnum {
    fn into(self) -> MeshAdvBlendMethod {
        match self {
            MeshAdvBlendMethodEnum::Opaque => MeshAdvBlendMethod::Opaque,
            MeshAdvBlendMethodEnum::AlphaClip => MeshAdvBlendMethod::AlphaClip,
            MeshAdvBlendMethodEnum::AlphaBlend => MeshAdvBlendMethod::AlphaBlend,
        }
    }
}

#[cfg(all(not(feature = "basic-pipeline"), feature = "legion"))]
impl Into<MeshAdvShadowMethod> for MeshAdvShadowMethodEnum {
    fn into(self) -> MeshAdvShadowMethod {
        match self {
            MeshAdvShadowMethodEnum::None => MeshAdvShadowMethod::None,
            MeshAdvShadowMethodEnum::Opaque => MeshAdvShadowMethod::Opaque,
        }
    }
}
