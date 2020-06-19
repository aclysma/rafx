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
use crate::resource_managers::{PipelineSwapchainInfo, DynDescriptorSet, DescriptorSetArc, DescriptorSetAllocatorRef, MeshInfo, ResourceManager, ResourceArc};
use crate::pipeline::pipeline::MaterialAsset;
use crate::pipeline::gltf::MeshAsset;
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
pub struct MeshPerFrameShaderParam {
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


// A mesh that, aside from moving around, does not change. (i.e. no material changes)
pub struct StaticMeshInstance {
    // Contains buffers, where to bind within the buffers
    pub mesh_info: MeshInfo,

    // Dynamic descriptor for position/view. These are bound to layout 2.
    // These really should be per-view so there probably needs to be a better way of handling this
    pub per_object_descriptor_set: DynDescriptorSet,

    // world-space transform (position/rotation/translation)
    pub world_transform: glam::Mat4,
}

impl StaticMeshInstance {
    pub fn new(
        resource_manager: &mut ResourceManager,
        mesh: &Handle<MeshAsset>,
        mesh_material: &Handle<MaterialAsset>,
        position: glam::Vec3,
    ) -> VkResult<Self> {
        let mesh_info = resource_manager.get_mesh_info(mesh);
        let object_descriptor_set = resource_manager.get_descriptor_set_info(mesh_material, 0, 2);
        let mut descriptor_set_allocator = resource_manager.create_descriptor_set_allocator();
        let per_object_descriptor_set = descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(&object_descriptor_set.descriptor_set_layout)?;

        let world_transform = glam::Mat4::from_translation(position);

        Ok(StaticMeshInstance {
            mesh_info,
            per_object_descriptor_set,
            world_transform
        })
    }

    pub fn set_view_proj(
        &mut self,
        view: glam::Mat4,
        proj: glam::Mat4,
        resource_manager: &mut ResourceManager
    ) {
        let model_view = view * self.world_transform;
        let model_view_proj = proj * model_view;

        let per_object_param = MeshPerObjectShaderParam {
            model_view,
            model_view_proj
        };

        let mut descriptor_set_allocator = resource_manager.create_descriptor_set_allocator();
        self.per_object_descriptor_set.set_buffer_data(0, &per_object_param);
        self.per_object_descriptor_set.flush(&mut descriptor_set_allocator);
    }
}




pub fn create_mesh_extract_job(
    device_context: VkDeviceContext,
    descriptor_set_allocator: DescriptorSetAllocatorRef,
    pipeline_info: PipelineSwapchainInfo,
    mesh_material: &Handle<MaterialAsset>,
    descriptor_set_per_pass: DescriptorSetArc,
) -> Box<dyn ExtractJob<RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext>> {
    Box::new(DefaultExtractJob::new(MeshExtractJobImpl::new(
        device_context,
        descriptor_set_allocator,
        pipeline_info,
        mesh_material,
        descriptor_set_per_pass,
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
    pub per_material_descriptor: DescriptorSetArc,
}

#[derive(Debug)]
pub struct ExtractedViewNodeMeshData {
    pub per_instance_descriptor: DescriptorSetArc,
    pub frame_node_index: FrameNodeIndex,
}