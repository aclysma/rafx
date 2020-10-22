use renderer::nodes::{
    RenderFeature, RenderFeatureIndex, DefaultExtractJob, ExtractJob, GenericRenderNodeHandle,
    RenderNodeSet, RenderNodeCount,
};
use crate::render_contexts::{RenderJobExtractContext, RenderJobWriteContext, RenderJobPrepareContext};
use renderer::base::slab::{DropSlabKey, DropSlab};
use std::convert::TryInto;
use atelier_assets::loader::handle::Handle;
use renderer::assets::MaterialAsset;
use renderer::assets::ImageAsset;

mod extract;
use extract::SpriteExtractJobImpl;

mod prepare;

mod write;
use write::SpriteCommandWriter;
use renderer::vulkan::VkDeviceContext;
use renderer::assets::resources::{
    DescriptorSetArc, DescriptorSetAllocatorRef, ResourceArc, GraphicsPipelineResource,
};

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
    pipeline_info: ResourceArc<GraphicsPipelineResource>,
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
    pub position: glam::Vec3,
    pub alpha: f32,
    pub image: Handle<ImageAsset>,
}

#[derive(Clone)]
pub struct SpriteRenderNodeHandle(pub DropSlabKey<SpriteRenderNode>);

impl SpriteRenderNodeHandle {
    pub fn as_raw_generic_handle(&self) -> GenericRenderNodeHandle {
        GenericRenderNodeHandle::new(
            <SpriteRenderFeature as RenderFeature>::feature_index(),
            self.0.index(),
        )
    }
}

impl Into<GenericRenderNodeHandle> for SpriteRenderNodeHandle {
    fn into(self) -> GenericRenderNodeHandle {
        self.as_raw_generic_handle()
    }
}

#[derive(Default)]
pub struct SpriteRenderNodeSet {
    sprites: DropSlab<SpriteRenderNode>,
}

impl SpriteRenderNodeSet {
    pub fn register_sprite(
        &mut self,
        node: SpriteRenderNode,
    ) -> SpriteRenderNodeHandle {
        SpriteRenderNodeHandle(self.sprites.allocate(node))
    }

    pub fn get_mut(
        &mut self,
        handle: &SpriteRenderNodeHandle,
    ) -> Option<&mut SpriteRenderNode> {
        self.sprites.get_mut(&handle.0)
    }

    pub fn update(&mut self) {
        self.sprites.process_drops();
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

renderer::declare_render_feature!(SpriteRenderFeature, SPRITE_FEATURE_INDEX);

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
