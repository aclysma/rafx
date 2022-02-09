use distill::loader::LoadHandle;
use rafx::api::RafxResult;
use rafx::assets::{AssetManager, DefaultAssetTypeHandler, DefaultAssetTypeLoadHandler};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use type_uuid::*;

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum AnimInterpolationMode {
    Linear,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BoneChannelVec3 {
    pub min_frame: u32,
    pub max_frame: u32,
    //pub interpolation: Vec<AnimInterpolationMode>,
    pub values: Vec<glam::Vec3>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BoneChannelQuat {
    pub min_frame: u32,
    pub max_frame: u32,
    //pub interpolation: Vec<AnimInterpolationMode>,
    pub values: Vec<glam::Quat>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct BoneChannelGroup {
    pub position: Option<BoneChannelVec3>,
    pub rotation: Option<BoneChannelQuat>,
    pub scale: Option<BoneChannelVec3>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnimClip {
    pub name: String,
    pub bone_channel_groups: Vec<BoneChannelGroup>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Bone {
    pub name: String,
    // _rel = relative to parent
    pub position_rel: glam::Vec3,
    pub rotation_rel: glam::Quat,
    pub parent: i16,
    pub chain_depth: i16,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
}

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone)]
#[uuid = "5ee97f08-d8a7-45d3-96bb-412831e8bcb6"]
pub struct AnimAssetData {
    pub skeleton: Skeleton,
    pub clips: Vec<AnimClip>,
}

#[derive(Debug)]
pub struct AnimAssetInner {
    pub skeleton: Arc<Skeleton>,
    pub clips: Vec<Arc<AnimClip>>,
}

#[derive(TypeUuid, Clone, Debug)]
#[uuid = "f5575dbe-a482-48a4-a1f8-2916f75f1c71"]
pub struct AnimAsset {
    inner: Arc<AnimAssetInner>,
}

impl AnimAsset {
    pub fn skeleton(&self) -> &Arc<Skeleton> {
        &self.inner.skeleton
    }

    pub fn clip(
        &self,
        index: usize,
    ) -> &Arc<AnimClip> {
        &self.inner.clips[index]
    }
}

pub struct AnimLoadHandler;

impl DefaultAssetTypeLoadHandler<AnimAssetData, AnimAsset> for AnimLoadHandler {
    #[profiling::function]
    fn load(
        _asset_manager: &mut AssetManager,
        anim_asset: AnimAssetData,
        _load_handle: LoadHandle,
    ) -> RafxResult<AnimAsset> {
        let skeleton = Arc::new(anim_asset.skeleton);
        let clips = anim_asset.clips.into_iter().map(|x| Arc::new(x)).collect();

        let inner = AnimAssetInner { skeleton, clips };

        Ok(AnimAsset {
            inner: Arc::new(inner),
        })
    }
}

pub type AnimAssetType = DefaultAssetTypeHandler<AnimAssetData, AnimAsset, AnimLoadHandler>;
