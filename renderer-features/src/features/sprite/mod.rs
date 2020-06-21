use renderer_base::{RenderFeature, RenderFeatureIndex, DefaultExtractJob, ExtractJob, GenericRenderNodeHandle, RenderNodeSet, RenderNodeCount};
use std::sync::atomic::{Ordering, AtomicI32};
use glam::f32::Vec3;
use crate::{RenderJobExtractContext, RenderJobWriteContext, DemoPrepareContext, RenderJobPrepareContext};
use legion::prelude::Entity;
use renderer_base::slab::{RawSlabKey, RawSlab};
use std::convert::TryInto;
use atelier_assets::loader::handle::Handle;

mod extract;
use extract::SpriteExtractJobImpl;

mod prepare;
use prepare::SpritePrepareJobImpl;

mod write;
use write::SpriteCommandWriter;
use renderer_shell_vulkan::VkDeviceContext;
use ash::vk;
use renderer_assets::resource_managers::{PipelineSwapchainInfo, DynDescriptorSet, DescriptorSetArc, DescriptorSetAllocatorRef};
use renderer_assets::pipeline::pipeline::MaterialAsset;

/// Per-pass "global" data
#[derive(Clone, Debug, Copy)]
struct SpriteUniformBufferObject {
    // View and projection matrices
    view_proj: [[f32; 4]; 4],
}

/// Vertex format for vertices sent to the GPU
#[derive(Clone, Debug, Copy)]
#[repr(C)]
pub struct SpriteVertex {
    pub pos: [f32; 2],
    pub tex_coord: [f32; 2],
    //color: [u8; 4],
}

/// Used as static data to represent a quad
#[derive(Clone, Debug, Copy)]
struct QuadVertex {
    pos: [f32; 3],
    tex_coord: [f32; 2],
}

/// Static data the represents a "unit" quad
const QUAD_VERTEX_LIST: [QuadVertex; 4] = [
    QuadVertex {
        pos: [-0.5, -0.5, 0.0],
        tex_coord: [1.0, 0.0],
    },
    QuadVertex {
        pos: [0.5, -0.5, 0.0],
        tex_coord: [0.0, 0.0],
    },
    QuadVertex {
        pos: [0.5, 0.5, 0.0],
        tex_coord: [0.0, 1.0],
    },
    QuadVertex {
        pos: [-0.5, 0.5, 0.0],
        tex_coord: [1.0, 1.0],
    },
];

/// Draw order of QUAD_VERTEX_LIST
const QUAD_INDEX_LIST: [u16; 6] = [0, 1, 2, 2, 3, 0];

pub fn create_sprite_extract_job(
    device_context: VkDeviceContext,
    descriptor_set_allocator: DescriptorSetAllocatorRef,
    pipeline_info: PipelineSwapchainInfo,
    sprite_material: &Handle<MaterialAsset>,
) -> Box<dyn ExtractJob<RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext>> {
    Box::new(DefaultExtractJob::new(SpriteExtractJobImpl::new(
        device_context,
        descriptor_set_allocator,
        pipeline_info,
        sprite_material,
    )))
}

//
// This is boiler-platish
//
pub struct SpriteRenderNode {
    pub entity: Entity, // texture
}

#[derive(Copy, Clone)]
pub struct SpriteRenderNodeHandle(pub RawSlabKey<SpriteRenderNode>);

impl Into<GenericRenderNodeHandle> for SpriteRenderNodeHandle {
    fn into(self) -> GenericRenderNodeHandle {
        GenericRenderNodeHandle::new(
            <SpriteRenderFeature as RenderFeature>::feature_index(),
            self.0.index(),
        )
    }
}

#[derive(Default)]
pub struct SpriteRenderNodeSet {
    sprites: RawSlab<SpriteRenderNode>,
}

impl SpriteRenderNodeSet {
    pub fn register_sprite(
        &mut self,
        node: SpriteRenderNode,
    ) -> SpriteRenderNodeHandle {
        SpriteRenderNodeHandle(self.sprites.allocate(node))
    }

    pub fn register_sprite_with_handle<F: FnMut(SpriteRenderNodeHandle) -> SpriteRenderNode>(
        &mut self,
        mut f: F,
    ) -> SpriteRenderNodeHandle {
        SpriteRenderNodeHandle(
            self.sprites
                .allocate_with_key(|handle| (f)(SpriteRenderNodeHandle(handle))),
        )
    }

    pub fn unregister_sprite(
        &mut self,
        handle: SpriteRenderNodeHandle,
    ) {
        self.sprites.free(handle.0);
    }
}

impl RenderNodeSet for SpriteRenderNodeSet {
    fn feature_index(&self) -> RenderFeatureIndex {
        SpriteRenderFeature::feature_index()
    }

    fn max_render_node_count(&self) -> RenderNodeCount {
        self.sprites.storage_size() as RenderNodeCount
    }
}

//
// This is boilerplate that could be macro'd
//
static SPRITE_FEATURE_INDEX: AtomicI32 = AtomicI32::new(-1);

pub struct SpriteRenderFeature;

impl RenderFeature for SpriteRenderFeature {
    fn set_feature_index(index: RenderFeatureIndex) {
        SPRITE_FEATURE_INDEX.store(index.try_into().unwrap(), Ordering::Release);
    }

    fn feature_index() -> RenderFeatureIndex {
        SPRITE_FEATURE_INDEX.load(Ordering::Acquire) as RenderFeatureIndex
    }

    fn feature_debug_name() -> &'static str {
        "SpriteRenderFeature"
    }
}

#[derive(Debug)]
pub(self) struct ExtractedSpriteData {
    position: glam::Vec3,
    texture_size: glam::Vec2,
    scale: f32,
    rotation: f32,
    alpha: f32,
    texture_descriptor_set: DescriptorSetArc, //TODO: I'd prefer to use something ref-counted
}

#[derive(Debug)]
pub struct SpriteDrawCall {
    index_buffer_first_element: u16,
    index_buffer_count: u16,
    texture_descriptor_set: DescriptorSetArc,
}