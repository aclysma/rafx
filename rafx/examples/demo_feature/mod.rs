use glam::f32::Vec3;
use rafx_base::slab::{DropSlab, DropSlabKey};
use rafx_nodes::{
    ExtractJob, FrameNodeIndex, GenericRenderNodeHandle, RenderFeature, RenderFeatureIndex,
    RenderNodeCount, RenderNodeSet, ViewNodeIndex,
};
use std::convert::TryInto;

mod extract;
use extract::DemoExtractJob;

mod prepare;
mod write;

use crate::{DemoExtractContext, DemoPrepareContext, DemoWriteContext};

pub fn create_demo_extract_job(
) -> Box<dyn ExtractJob<DemoExtractContext, DemoPrepareContext, DemoWriteContext>> {
    Box::new(DemoExtractJob::default())
}

//
// This is boiler-platish
//
pub struct DemoRenderNode {
    pub position: glam::Vec3,
    pub alpha: f32,
}

#[derive(Clone)]
pub struct DemoRenderNodeHandle(pub DropSlabKey<DemoRenderNode>);

impl DemoRenderNodeHandle {
    pub fn as_raw_generic_handle(&self) -> GenericRenderNodeHandle {
        GenericRenderNodeHandle::new(
            <DemoRenderFeature as RenderFeature>::feature_index(),
            self.0.index(),
        )
    }
}

impl Into<GenericRenderNodeHandle> for DemoRenderNodeHandle {
    fn into(self) -> GenericRenderNodeHandle {
        self.as_raw_generic_handle()
    }
}

#[derive(Default)]
pub struct DemoRenderNodeSet {
    demos: DropSlab<DemoRenderNode>,
}

impl DemoRenderNodeSet {
    #[allow(dead_code)]
    pub fn register_demo_component(
        &mut self,
        node: DemoRenderNode,
    ) -> DemoRenderNodeHandle {
        DemoRenderNodeHandle(self.demos.allocate(node))
    }

    pub fn get_mut(
        &mut self,
        handle: &DemoRenderNodeHandle,
    ) -> Option<&mut DemoRenderNode> {
        self.demos.get_mut(&handle.0)
    }

    pub fn update(&mut self) {
        self.demos.process_drops();
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

rafx::declare_render_feature!(DemoRenderFeature, DEMO_FEATURE_INDEX);

#[derive(Debug, Clone)]
pub(self) struct ExtractedPerFrameNodeDemoData {
    position: Vec3,
    alpha: f32,
}

pub(self) struct ExtractedPerViewNodeDemoData {
    position: Vec3,
    alpha: f32,
}

pub(self) struct PreparedPerSubmitNodeDemoData {
    #[allow(dead_code)]
    position: Vec3,
    #[allow(dead_code)]
    alpha: f32,
    frame_node_index: FrameNodeIndex,
    view_node_index: ViewNodeIndex,
}
