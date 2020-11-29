use crate::game_asset_lookup::MeshAsset;
use crate::render_contexts::{
    RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext,
};
use atelier_assets::loader::handle::Handle;
use renderer::assets::assets::MaterialPass;
use renderer::base::slab::{DropSlab, DropSlabKey};
use renderer::nodes::RenderView;
use renderer::nodes::{
    ExtractJob, FrameNodeIndex, GenericRenderNodeHandle, RenderFeature, RenderFeatureIndex,
    RenderNodeCount, RenderNodeSet,
};
use renderer::resources::{ImageViewResource, ResourceArc};
use std::convert::TryInto;

mod extract;
use extract::MeshExtractJob;

mod prepare;

mod write;
use renderer::resources::DescriptorSetArc;
use write::MeshCommandWriter;

const PER_VIEW_DESCRIPTOR_SET_INDEX: u32 =
    shaders::mesh_frag::PER_VIEW_DATA_DESCRIPTOR_SET_INDEX as u32;
const PER_MATERIAL_DESCRIPTOR_SET_INDEX: u32 =
    shaders::mesh_frag::PER_MATERIAL_DATA_DESCRIPTOR_SET_INDEX as u32;
const PER_INSTANCE_DESCRIPTOR_SET_INDEX: u32 =
    shaders::mesh_frag::PER_OBJECT_DATA_DESCRIPTOR_SET_INDEX as u32;

use crate::components::{
    DirectionalLightComponent, PointLightComponent, PositionComponent, SpotLightComponent,
};
use fnv::FnvHashMap;
pub use shaders::mesh_frag::PerObjectDataUniform as MeshPerObjectFragmentShaderParam;
pub use shaders::mesh_frag::PerViewDataUniform as MeshPerViewFragmentShaderParam;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum LightId {
    PointLight(legion::Entity),
    SpotLight(legion::Entity),
    DirectionalLight(legion::Entity),
}

pub struct ShadowMapData {
    pub shadow_map_lookup: FnvHashMap<LightId, usize>,
    pub shadow_map_render_views: Vec<RenderView>,
    pub shadow_map_images: Vec<ResourceArc<ImageViewResource>>,
}

pub struct PreparedDirectionalLight {
    light: DirectionalLightComponent,
    shadow_map_index: Option<usize>,
}

pub struct PreparedPointLight {
    light: PointLightComponent,
    position: PositionComponent,
    shadow_map_index: Option<usize>,
}

pub struct PreparedSpotLight {
    light: SpotLightComponent,
    position: PositionComponent,
    shadow_map_index: Option<usize>,
}

pub fn create_mesh_extract_job(
    shadow_map_data: ShadowMapData
) -> Box<dyn ExtractJob<RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext>> {
    Box::new(MeshExtractJob { shadow_map_data })
}

//
// This is boiler-platish
//
pub struct MeshRenderNode {
    pub mesh: Option<Handle<MeshAsset>>,
    pub transform: glam::Mat4,
}

#[derive(Clone)]
pub struct MeshRenderNodeHandle(pub DropSlabKey<MeshRenderNode>);

impl MeshRenderNodeHandle {
    pub fn as_raw_generic_handle(&self) -> GenericRenderNodeHandle {
        GenericRenderNodeHandle::new(
            <MeshRenderFeature as RenderFeature>::feature_index(),
            self.0.index(),
        )
    }
}

impl Into<GenericRenderNodeHandle> for MeshRenderNodeHandle {
    fn into(self) -> GenericRenderNodeHandle {
        self.as_raw_generic_handle()
    }
}

#[derive(Default)]
pub struct MeshRenderNodeSet {
    meshes: DropSlab<MeshRenderNode>,
}

impl MeshRenderNodeSet {
    pub fn register_mesh(
        &mut self,
        node: MeshRenderNode,
    ) -> MeshRenderNodeHandle {
        MeshRenderNodeHandle(self.meshes.allocate(node))
    }

    pub fn get_mut(
        &mut self,
        handle: &MeshRenderNodeHandle,
    ) -> Option<&mut MeshRenderNode> {
        self.meshes.get_mut(&handle.0)
    }

    pub fn update(&mut self) {
        self.meshes.process_drops();
    }
}

impl RenderNodeSet for MeshRenderNodeSet {
    fn feature_index(&self) -> RenderFeatureIndex {
        MeshRenderFeature::feature_index()
    }

    fn max_render_node_count(&self) -> RenderNodeCount {
        self.meshes.storage_size() as RenderNodeCount
    }
}

renderer::declare_render_feature!(MeshRenderFeature, MESH_FEATURE_INDEX);

pub struct ExtractedFrameNodeMeshData {
    world_transform: glam::Mat4,
    mesh_asset: MeshAsset,
}

impl std::fmt::Debug for ExtractedFrameNodeMeshData {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("ExtractedFrameNodeMeshData")
            .field("world_transform", &self.world_transform)
            .finish()
    }
}

pub struct PreparedSubmitNodeMeshData {
    material_pass: MaterialPass,
    per_view_descriptor_set: Option<DescriptorSetArc>,
    per_material_descriptor_set: Option<DescriptorSetArc>,
    per_instance_descriptor_set: DescriptorSetArc,
    // we can get the mesh via the frame node index
    frame_node_index: FrameNodeIndex,
    mesh_part_index: usize,
}

impl std::fmt::Debug for PreparedSubmitNodeMeshData {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("PreparedSubmitNodeMeshData")
            .field("frame_node_index", &self.frame_node_index)
            .field("mesh_part_index", &self.mesh_part_index)
            .finish()
    }
}
