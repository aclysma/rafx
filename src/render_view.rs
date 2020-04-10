
use glam::Mat4;
use crate::frame_packet::FramePacket;
//use crate::visibility::StaticVisibilityResult;
//use crate::visibility::DynamicVisibilityResult;
use crate::visibility::VisibilityResult;
use crate::{RenderNodeSet, RenderPhase, GenericRenderNodeHandle};
use crate::registry::{RenderPhaseMaskInnerType, MAX_RENDER_PHASE_COUNT};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

#[derive(Default)]
pub struct RenderPhaseMaskBuilder(RenderPhaseMaskInnerType);

impl RenderPhaseMaskBuilder {
    pub fn add_render_phase<T: RenderPhase>(mut self) -> RenderPhaseMaskBuilder {
        let index = T::render_phase_index();
        assert!(index < MAX_RENDER_PHASE_COUNT);
        self.0 |= 1<<T::render_phase_index();
        self
    }

    pub fn build(self) -> RenderPhaseMask {
        RenderPhaseMask(self.0)
    }
}

#[derive(Copy, Clone)]
pub struct RenderPhaseMask(RenderPhaseMaskInnerType);

#[derive(Default)]
pub struct RenderViewSet {
    view_count: AtomicUsize
}

impl RenderViewSet {
    pub fn create_view(&self, view_proj: Mat4, render_stage_mask: RenderPhaseMask, debug_name: String) -> RenderView {
        let view_index = self.view_count.fetch_add(1, Ordering::Release);
        RenderView::new(view_index, view_proj, render_stage_mask, debug_name)
    }

    pub fn view_count(&self) -> usize {
        self.view_count.load(Ordering::Acquire)
    }
}

////////////////// Views //////////////////
pub struct RenderView {
    view_proj: Mat4,
    view_index: usize,
    render_stage_mask: RenderPhaseMask,
    debug_name: String

    //visibility_results: Vec<Vec<GenericRenderNodeHandle>>,
}

impl RenderView {
    pub fn new(view_index: usize, view_proj: Mat4, render_stage_mask: RenderPhaseMask, debug_name: String) -> RenderView {
        log::debug!("Allocate view {} {}", debug_name, view_index);
        //let view_index = frame_packet.allocate_view_packet();
        Self {
            view_proj,
            view_index,
            render_stage_mask,
            debug_name
            //visibility_results: Default::default(),
        }
    }

    pub fn view_index(&self) -> usize {
        self.view_index
    }

    pub fn debug_name(&self) -> &str {
        &self.debug_name
    }

    // pub fn add_static_visibility_results(&mut self) {
    //
    // }
    //
    // pub fn add_dynamic_visibility_results(&mut self) {
    //
    // }

    // pub fn allocate_frame_packet_nodes(
    //     &self,
    //     render_node_set: &RenderNodeSet,
    //     frame_packet: &FramePacket,
    //     static_visibility: &StaticVisibilityResult,
    //     dynamic_visibility: &DynamicVisibilityResult)
    // {
    //     let view_packet = frame_packet.view_packet(self.view_index);
    //
    //     //let handle_bins =
    //
    //     // Compute views
    //     // Kick off extract job per view
    //     //  - Produce list of visible objects for the view
    //     //  - Create per-view nodes
    //     //  - Create per-frame nodes
    //     //  (sync point here to wait for all views to be done?)
    //     //  - frame extract entry point
    //     //  - extract per-frame nodes
    //     //  - extract per-view nodes
    //
    //     // Are there per object nodes?
    //
    //
    //     for handle in &static_visibility.handles {
    //         let frame_node_index = frame_packet.append_frame_node(*handle);
    //         let view_node_index = view_packet.append_view_node(*handle, frame_node_index);
    //     }
    //
    //     for handle in &dynamic_visibility.handles {
    //         let frame_node_index = frame_packet.append_frame_node(*handle);
    //         let view_node_index = view_packet.append_view_node(*handle, frame_node_index);
    //     }
    // }
    //
    // pub fn extract(
    //     &self,
    //     frame_packet: &mut FramePacket,
    //     //world: &World
    // ) {
    //     // Extract all the data into the frame packet
    // }
}
