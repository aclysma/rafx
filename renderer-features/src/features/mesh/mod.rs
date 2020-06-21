use renderer_base::{RenderFeature, RenderFeatureIndex, DefaultExtractJob, ExtractJob, GenericRenderNodeHandle, RenderNodeSet, RenderNodeCount, FrameNodeIndex};
use std::sync::atomic::{Ordering, AtomicI32};
use glam::f32::Vec3;
use crate::{RenderJobExtractContext, RenderJobWriteContext, DemoPrepareContext, RenderJobPrepareContext};
use legion::prelude::Entity;
use renderer_base::slab::{RawSlabKey, RawSlab};
use std::convert::TryInto;
use atelier_assets::loader::handle::Handle;

mod extract;
use extract::MeshExtractJobImpl;

mod prepare;
use prepare::MeshPrepareJobImpl;

mod write;
use write::MeshCommandWriter;
use renderer_shell_vulkan::{VkDeviceContext, VkBufferRaw};
use ash::vk;
use renderer_resources::resource_managers::{PipelineSwapchainInfo, DynDescriptorSet, DescriptorSetArc, DescriptorSetAllocatorRef, MeshInfo, ResourceManager, ResourceArc};
use renderer_assets::pipeline::pipeline::MaterialAsset;
use renderer_assets::pipeline::gltf::MeshAsset;
use ash::prelude::VkResult;


// Represents the data uploaded to the GPU to represent a single point light
#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct PointLight {
    pub position_ws: glam::Vec3, // +0
    pub position_vs: glam::Vec3, // +16
    pub color: glam::Vec4, // +32
    pub range: f32, // +48
    pub intensity: f32, // +52
} // 4*16 = 64 bytes

// Represents the data uploaded to the GPU to represent a single directional light
#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct DirectionalLight {
    pub direction_ws: glam::Vec3, // +0
    pub direction_vs: glam::Vec3, // +16
    pub color: glam::Vec4, // +32
    pub intensity: f32, // +48
} // 4*16 = 64 bytes

// Represents the data uploaded to the GPU to represent a single spot light
#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct SpotLight {
    pub position_ws: glam::Vec3, // +0
    pub direction_ws: glam::Vec3, // +16
    pub position_vs: glam::Vec3, // +32
    pub direction_vs: glam::Vec3, // +48
    pub color: glam::Vec4, // +64
    pub spotlight_half_angle: f32, //+80
    pub range: f32, // +84
    pub intensity: f32, // +88
} // 6*16 = 96 bytes

// Represents the data uploaded to the GPU to provide all data necessary to render meshes
//TODO: Remove view/proj, they aren't being used. Add ambient light constant
#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct MeshPerViewShaderParam {
    pub ambient_light: glam::Vec4, // +0
    pub point_light_count: u32, // +16
    pub directional_light_count: u32, // 20
    pub spot_light_count: u32, // +24
    pub point_lights: [PointLight; 16], // +32 (64*16 = 1024),
    pub directional_lights: [DirectionalLight; 16], // +1056 (64*16 = 1024),
    pub spot_lights: [SpotLight; 16], // +2080 (96*16 = 1536)
} // 3616 bytes

#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct MeshPerObjectShaderParam {
    pub model_view: glam::Mat4, // +0
    pub model_view_proj: glam::Mat4, // +64
} // 128 bytes

pub fn create_mesh_extract_job(
    device_context: VkDeviceContext,
    descriptor_set_allocator: DescriptorSetAllocatorRef,
    pipeline_info: PipelineSwapchainInfo,
    mesh_material: &Handle<MaterialAsset>,
) -> Box<dyn ExtractJob<RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext>> {
    Box::new(DefaultExtractJob::new(MeshExtractJobImpl::new(
        device_context,
        descriptor_set_allocator,
        pipeline_info,
        mesh_material,
    )))
}

//
// This is boiler-platish
//
pub struct MeshRenderNode {
    pub entity: Entity, // texture
}

#[derive(Copy, Clone)]
pub struct MeshRenderNodeHandle(pub RawSlabKey<MeshRenderNode>);

impl Into<GenericRenderNodeHandle> for MeshRenderNodeHandle {
    fn into(self) -> GenericRenderNodeHandle {
        GenericRenderNodeHandle::new(
            <MeshRenderFeature as RenderFeature>::feature_index(),
            self.0.index(),
        )
    }
}

#[derive(Default)]
pub struct MeshRenderNodeSet {
    meshes: RawSlab<MeshRenderNode>,
}

impl MeshRenderNodeSet {
    pub fn register_mesh(
        &mut self,
        node: MeshRenderNode,
    ) -> MeshRenderNodeHandle {
        MeshRenderNodeHandle(self.meshes.allocate(node))
    }

    pub fn register_mesh_with_handle<F: FnMut(MeshRenderNodeHandle) -> MeshRenderNode>(
        &mut self,
        mut f: F,
    ) -> MeshRenderNodeHandle {
        MeshRenderNodeHandle(
            self.meshes
                .allocate_with_key(|handle| (f)(MeshRenderNodeHandle(handle))),
        )
    }

    pub fn unregister_mesh(
        &mut self,
        handle: MeshRenderNodeHandle,
    ) {
        self.meshes.free(handle.0);
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

//
// This is boilerplate that could be macro'd
//
static MESH_FEATURE_INDEX: AtomicI32 = AtomicI32::new(-1);

pub struct MeshRenderFeature;

impl RenderFeature for MeshRenderFeature {
    fn set_feature_index(index: RenderFeatureIndex) {
        MESH_FEATURE_INDEX.store(index.try_into().unwrap(), Ordering::Release);
    }

    fn feature_index() -> RenderFeatureIndex {
        MESH_FEATURE_INDEX.load(Ordering::Acquire) as RenderFeatureIndex
    }

    fn feature_debug_name() -> &'static str {
        "MeshRenderFeature"
    }
}

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