use crate::slab::{RawSlabKey, RawSlab};
use crate::registry::RenderFeature;
use crate::registry::RenderFeatureIndex;
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicI32;
use crate::{FramePacket, GenericRenderNodeHandle, ExtractJob, DefaultExtractJob, DefaultExtractJobImpl, PrepareJob, RenderView, RenderNodeSet};
use std::convert::TryInto;
use crate::RenderFeatureExtractImpl;
use legion::prelude::World;

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






struct SpriteExtractJobImpl {
    //world: &'static World,
    //render_nodes: &'static SpriteRenderNodeSet
}

impl DefaultExtractJobImpl<World> for SpriteExtractJobImpl {
    fn extract_begin(
        &self,
        source: &World,
    ) {
        log::debug!("extract_begin {}", self.feature_debug_name());
    }
    fn extract_frame_node(
        &self,
        source: &World,
        entity: u32,
    ) {
        log::debug!("extract_frame_node {}", self.feature_debug_name());
    }

    fn extract_view_node(
        &self,
        source: &World,
        entity: u32,
        view: u32,
    ) {
        log::debug!("extract_view_nodes {}", self.feature_debug_name());
    }
    fn extract_view_finalize(
        &self,
        source: &World,
        view: u32,
    ) {
        log::debug!("extract_view_finalize {}", self.feature_debug_name());
    }
    fn extract_frame_finalize(
        self,
        source: &World,
    ) -> Box<PrepareJob> {
        log::debug!("extract_frame_finalize {}", self.feature_debug_name());
        Box::new(SpritePrepareJob { })
    }

    fn feature_debug_name(&self) -> &'static str {
        SpriteRenderFeature::feature_debug_name()
    }
}

pub struct SpriteExtractJob {
    inner: Box<DefaultExtractJob<World, SpriteExtractJobImpl>>
}

impl SpriteExtractJob {
    pub fn new(/*world: &World, render_nodes: &SpriteRenderNodeSet*/) -> Self {
        // let world = unsafe {
        //     force_to_static_lifetime(world)
        // };
        //
        // let render_nodes = unsafe {
        //     force_to_static_lifetime(render_nodes)
        // };

        let job_impl = SpriteExtractJobImpl {
            //world,
            //render_nodes
        };

        SpriteExtractJob {
            inner: Box::new(DefaultExtractJob::new(job_impl))
        }
    }
}

impl ExtractJob<World> for SpriteExtractJob {
    fn extract(self: Box<Self>, source: &World, frame_packet: &FramePacket, views: &[&RenderView]) -> Box<PrepareJob> {
        self.inner.extract(source, frame_packet, views)
    }

    fn feature_debug_name(&self) -> &'static str {
        self.inner.feature_debug_name()
        //ExtractJob::<World>::feature_debug_name(&*self.inner)
    }
}

//
// unsafe fn force_to_static_lifetime<T>(value: &T) -> &'static T{
//     std::mem::transmute(value)
// }




struct SpritePrepareJob {

}

impl PrepareJob for SpritePrepareJob {
    fn prepare(self) {

    }
}


pub struct SpriteRenderNode {
    // entity handle
    // texture
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




pub struct SpriteRenderNodeSet {
    sprites: RawSlab<SpriteRenderNode>
}

impl SpriteRenderNodeSet {
    pub fn new() -> Self {
        SpriteRenderNodeSet {
            sprites: Default::default(),
        }
    }

    pub fn register_sprite(
        &mut self,
        node: SpriteRenderNode,
    ) -> SpriteRenderNodeHandle {
        SpriteRenderNodeHandle(self.sprites.allocate(node))
    }

    pub fn unregister_sprite(
        &mut self,
        handle: SpriteRenderNodeHandle,
    ) {
        self.sprites.free(&handle.0);
    }
}

impl RenderNodeSet for SpriteRenderNodeSet {
    fn feature_index(&self) -> RenderFeatureIndex {
        SpriteRenderFeature::feature_index()
    }

    fn max_node_count(&self) -> usize {
        self.sprites.storage_size()
    }
}