use distill::loader::handle::Handle;
use rafx::base::slab::{DropSlab, DropSlabKey};
use rafx::framework::MaterialPass;
use rafx::nodes::RenderView;
use rafx::nodes::{
    ExtractJob, FrameNodeIndex, GenericRenderNodeHandle, RenderFeature, RenderFeatureIndex,
    RenderNodeCount, RenderNodeSet,
};
use std::convert::TryInto;

mod extract;
use extract::MeshExtractJob;

mod prepare;

mod write;

mod plugin;
pub use plugin::MeshRendererPlugin;

pub mod shadow_map_resource;

use rafx::framework::DescriptorSetArc;
use write::MeshCommandWriter;

const PER_VIEW_DESCRIPTOR_SET_INDEX: u32 =
    shaders::mesh_frag::PER_VIEW_DATA_DESCRIPTOR_SET_INDEX as u32;
const PER_MATERIAL_DESCRIPTOR_SET_INDEX: u32 =
    shaders::mesh_frag::PER_MATERIAL_DATA_DESCRIPTOR_SET_INDEX as u32;
const PER_INSTANCE_DESCRIPTOR_SET_INDEX: u32 =
    shaders::mesh_frag::PER_OBJECT_DATA_DESCRIPTOR_SET_INDEX as u32;

use crate::assets::gltf::MeshAsset;
use crate::components::{
    DirectionalLightComponent, PointLightComponent, PositionComponent, SpotLightComponent,
};
pub use shaders::mesh_frag::PerObjectDataUniform as MeshPerObjectFragmentShaderParam;
pub use shaders::mesh_frag::PerViewDataUniform as MeshPerViewFragmentShaderParam;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum LightId {
    PointLight(legion::Entity), // u32 is a face index
    SpotLight(legion::Entity),
    DirectionalLight(legion::Entity),
}

#[derive(Clone)]
pub enum ShadowMapRenderView {
    Single(RenderView), // width, height of texture
    Cube([RenderView; 6]),
}

pub struct ExtractedDirectionalLight {
    light: DirectionalLightComponent,
    entity: legion::Entity,
}

pub struct ExtractedPointLight {
    light: PointLightComponent,
    position: PositionComponent,
    entity: legion::Entity,
}

pub struct ExtractedSpotLight {
    light: SpotLightComponent,
    position: PositionComponent,
    entity: legion::Entity,
}

pub fn create_mesh_extract_job() -> Box<dyn ExtractJob> {
    Box::new(MeshExtractJob {})
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

rafx::declare_render_feature!(MeshRenderFeature, MESH_FEATURE_INDEX);

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
