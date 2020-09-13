use renderer_nodes::{
    RenderFeature, RenderFeatureIndex, DefaultExtractJob, ExtractJob, GenericRenderNodeHandle,
    RenderNodeSet, RenderNodeCount,
};
use std::sync::atomic::{Ordering, AtomicI32};
use glam::f32::Vec3;
use renderer_base::slab::{RawSlabKey, RawSlab};
use std::convert::TryInto;
use legion::*;

mod extract;
use extract::DemoExtractJobImpl;

mod prepare;
mod write;

use crate::{DemoExtractContext, DemoPrepareContext, DemoWriteContext};

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
    #[allow(dead_code)]
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

renderer::render_feature!(DemoRenderFeature, DEMO_FEATURE_INDEX);

#[derive(Debug, Clone)]
pub(self) struct ExtractedDemoData {
    position: Vec3,
    alpha: f32,
}
