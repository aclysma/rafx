use renderer_nodes::{
    RenderFeature, RenderFeatureIndex, DefaultExtractJob, ExtractJob, GenericRenderNodeHandle,
    RenderNodeSet, RenderNodeCount,
};
use std::sync::atomic::{Ordering, AtomicI32};
use glam::f32::Vec3;
use crate::{
    RenderJobExtractContext, DemoWriteContext, RenderJobPrepareContext, DemoPrepareContext,
    DemoExtractContext,
};
use legion::prelude::Entity;
use renderer_base::slab::{RawSlabKey, RawSlab};
use std::convert::TryInto;

mod extract;
use extract::DemoExtractJobImpl;

mod prepare;
use prepare::DemoPrepareJobImpl;

mod write;
use write::DemoCommandWriter;

pub fn create_demo_extract_job(
) -> Box<dyn ExtractJob<DemoExtractContext, DemoPrepareContext, DemoWriteContext>> {
    Box::new(DefaultExtractJob::new(DemoExtractJobImpl::default()))
}

//
// This is boiler-platish
//
pub struct DemoRenderNode {
    pub entity: Entity, // texture
}

#[derive(Copy, Clone)]
pub struct DemoRenderNodeHandle(pub RawSlabKey<DemoRenderNode>);

impl Into<GenericRenderNodeHandle> for DemoRenderNodeHandle {
    fn into(self) -> GenericRenderNodeHandle {
        GenericRenderNodeHandle::new(
            <DemoRenderFeature as RenderFeature>::feature_index(),
            self.0.index(),
        )
    }
}

#[derive(Default)]
pub struct DemoRenderNodeSet {
    demos: RawSlab<DemoRenderNode>,
}

impl DemoRenderNodeSet {
    pub fn new() -> Self {
        DemoRenderNodeSet {
            demos: Default::default(),
        }
    }

    pub fn register_demo_component(
        &mut self,
        node: DemoRenderNode,
    ) -> DemoRenderNodeHandle {
        DemoRenderNodeHandle(self.demos.allocate(node))
    }

    pub fn register_demo_component_with_handle<F: FnMut(DemoRenderNodeHandle) -> DemoRenderNode>(
        &mut self,
        mut f: F,
    ) -> DemoRenderNodeHandle {
        DemoRenderNodeHandle(
            self.demos
                .allocate_with_key(|handle| (f)(DemoRenderNodeHandle(handle))),
        )
    }

    pub fn unregister_demo_component(
        &mut self,
        handle: DemoRenderNodeHandle,
    ) {
        self.demos.free(handle.0);
    }
}

impl RenderNodeSet for DemoRenderNodeSet {
    fn feature_index(&self) -> RenderFeatureIndex {
        DemoRenderFeature::feature_index()
    }

    fn max_render_node_count(&self) -> RenderNodeCount {
        self.demos.storage_size() as RenderNodeCount
    }
}

//
// This is boilerplate that could be macro'd
//
static SPRITE_FEATURE_INDEX: AtomicI32 = AtomicI32::new(-1);

pub struct DemoRenderFeature;

impl RenderFeature for DemoRenderFeature {
    fn set_feature_index(index: RenderFeatureIndex) {
        SPRITE_FEATURE_INDEX.store(index.try_into().unwrap(), Ordering::Release);
    }

    fn feature_index() -> RenderFeatureIndex {
        SPRITE_FEATURE_INDEX.load(Ordering::Acquire) as RenderFeatureIndex
    }

    fn feature_debug_name() -> &'static str {
        "DemoRenderFeature"
    }
}

#[derive(Debug, Clone)]
pub(self) struct ExtractedDemoData {
    position: Vec3,
    alpha: f32,
}
