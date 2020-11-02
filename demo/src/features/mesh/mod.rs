use renderer::nodes::{
    RenderFeature, RenderFeatureIndex, ExtractJob, GenericRenderNodeHandle, RenderNodeSet,
    RenderNodeCount, FrameNodeIndex,
};
use crate::game_asset_lookup::MeshAsset;
use crate::render_contexts::{RenderJobExtractContext, RenderJobWriteContext, RenderJobPrepareContext};
use renderer::base::slab::{DropSlabKey, DropSlab};
use std::convert::TryInto;
use atelier_assets::loader::handle::Handle;
use renderer::assets::assets::MaterialPass;
use renderer::assets::resources::{ResourceArc, ImageViewResource};
use renderer::nodes::RenderView;

mod extract;
use extract::MeshExtractJob;

mod prepare;

mod write;
use write::MeshCommandWriter;
use renderer::assets::resources::{DescriptorSetArc};

const PER_VIEW_DESCRIPTOR_SET_INDEX: u32 = 0;
const PER_MATERIAL_DESCRIPTOR_SET_INDEX: u32 = 1;
const PER_INSTANCE_DESCRIPTOR_SET_INDEX: u32 = 2;

// Represents the data uploaded to the GPU to represent a single point light
#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct PointLight {
    pub position_ws: glam::Vec3, // +0
    pub position_vs: glam::Vec3, // +16
    pub color: glam::Vec4,       // +32
    pub range: f32,              // +48
    pub intensity: f32,          // +52
} // 4*16 = 64 bytes

// Represents the data uploaded to the GPU to represent a single directional light
#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct DirectionalLight {
    pub direction_ws: glam::Vec3, // +0
    pub direction_vs: glam::Vec3, // +16
    pub color: glam::Vec4,        // +32
    pub intensity: f32,           // +48
} // 4*16 = 64 bytes

// Represents the data uploaded to the GPU to represent a single spot light
#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct SpotLight {
    pub position_ws: glam::Vec3,   // +0
    pub direction_ws: glam::Vec3,  // +16
    pub position_vs: glam::Vec3,   // +32
    pub direction_vs: glam::Vec3,  // +48
    pub color: glam::Vec4,         // +64
    pub spotlight_half_angle: f32, //+80
    pub range: f32,                // +84
    pub intensity: f32,            // +88
} // 6*16 = 96 bytes

// Represents the data uploaded to the GPU to provide all data necessary to render meshes
//TODO: Remove view/proj, they aren't being used. Add ambient light constant
#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct MeshPerViewFragmentShaderParam {
    pub ambient_light: glam::Vec4,                  // +0
    pub point_light_count: u32,                     // +16
    pub directional_light_count: u32,               // 20
    pub spot_light_count: u32,                      // +24
    pub point_lights: [PointLight; 16],             // +32 (64*16 = 1024),
    pub directional_lights: [DirectionalLight; 16], // +1056 (64*16 = 1024),
    pub spot_lights: [SpotLight; 16],               // +2080 (96*16 = 1536)
} // 3616 bytes

#[derive(Default, Copy, Clone)]
//#[repr(C)]
#[repr(C)]
pub struct MeshPerFrameVertexShaderParam {
    pub shadow_map_view_proj: glam::Mat4, // +0
    pub shadow_map_light_dir: glam::Vec4, // +64
} // 80 bytes

#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct MeshPerObjectShaderParam {
    pub model: glam::Mat4,           // +0
    pub model_view: glam::Mat4,      // +64
    pub model_view_proj: glam::Mat4, // +128
} // 192 bytes

pub fn create_mesh_extract_job(
    shadow_map_image: ResourceArc<ImageViewResource>,
    shadow_map_view: RenderView,
) -> Box<dyn ExtractJob<RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext>> {
    Box::new(MeshExtractJob {
        shadow_map_image,
        shadow_map_view,
    })
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
    per_view_descriptor_set: DescriptorSetArc,
    per_material_descriptor_set: DescriptorSetArc,
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
