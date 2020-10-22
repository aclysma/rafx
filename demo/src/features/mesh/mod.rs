use renderer::nodes::{
    RenderFeature, RenderFeatureIndex, DefaultExtractJob, ExtractJob, GenericRenderNodeHandle,
    RenderNodeSet, RenderNodeCount, FrameNodeIndex,
};
use crate::game_asset_lookup::MeshAsset;
use crate::render_contexts::{RenderJobExtractContext, RenderJobWriteContext, RenderJobPrepareContext};
use renderer::base::slab::{DropSlabKey, DropSlab};
use std::convert::TryInto;
use atelier_assets::loader::handle::Handle;

mod extract;
use extract::MeshExtractJobImpl;

mod prepare;

mod write;
use write::MeshCommandWriter;
use renderer::vulkan::VkBufferRaw;
use renderer::assets::resources::{
    DescriptorSetArc, DescriptorSetAllocatorRef, ResourceArc, GraphicsPipelineResource,
};
use renderer::assets::MaterialAsset;

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
pub struct MeshPerViewShaderParam {
    pub ambient_light: glam::Vec4,                  // +0
    pub point_light_count: u32,                     // +16
    pub directional_light_count: u32,               // 20
    pub spot_light_count: u32,                      // +24
    pub point_lights: [PointLight; 16],             // +32 (64*16 = 1024),
    pub directional_lights: [DirectionalLight; 16], // +1056 (64*16 = 1024),
    pub spot_lights: [SpotLight; 16],               // +2080 (96*16 = 1536)
} // 3616 bytes

#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct MeshPerObjectShaderParam {
    pub model_view: glam::Mat4,      // +0
    pub model_view_proj: glam::Mat4, // +64
} // 128 bytes

pub fn create_mesh_extract_job(
    descriptor_set_allocator: DescriptorSetAllocatorRef,
    pipeline_info: ResourceArc<GraphicsPipelineResource>,
    mesh_material: &Handle<MaterialAsset>,
) -> Box<dyn ExtractJob<RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext>> {
    Box::new(DefaultExtractJob::new(MeshExtractJobImpl::new(
        descriptor_set_allocator,
        pipeline_info,
        mesh_material,
    )))
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

#[derive(Debug)]
pub struct ExtractedFrameNodeMeshData {
    world_transform: glam::Mat4,
    draw_calls: Vec<MeshDrawCall>,
    vertex_buffer: ResourceArc<VkBufferRaw>,
    index_buffer: ResourceArc<VkBufferRaw>,
}

#[derive(Debug)]
pub struct MeshDrawCall {
    pub vertex_buffer_offset_in_bytes: u32,
    pub vertex_buffer_size_in_bytes: u32,
    pub index_buffer_offset_in_bytes: u32,
    pub index_buffer_size_in_bytes: u32,
    pub per_material_descriptor: DescriptorSetArc, // set 1
}

#[derive(Debug)]
pub struct ExtractedViewNodeMeshData {
    pub per_instance_descriptor: DescriptorSetArc, // set 2
}

#[derive(Debug)]
pub struct PreparedViewNodeMeshData {
    pub per_instance_descriptor: DescriptorSetArc, // set 2
    pub frame_node_index: FrameNodeIndex,
    pub per_view_descriptor: DescriptorSetArc, // set 0
}
