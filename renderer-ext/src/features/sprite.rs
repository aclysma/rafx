use renderer_base::slab::{RawSlabKey, RawSlab};
use renderer_base::{RenderFeature, DefaultPrepareJobImpl, DefaultPrepareJob};
use renderer_base::RenderFeatureIndex;
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicI32;
use renderer_base::{
    FramePacket, GenericRenderNodeHandle, ExtractJob, PrepareJob, RenderView, RenderNodeSet,
};
use renderer_base::{DefaultExtractJob, DefaultExtractJobImpl};
use std::convert::TryInto;
use legion::prelude::{World, Read, IntoQuery};
use renderer_base::{PerFrameNode, PerViewNode};
use legion::entity::Entity;
use glam::Vec3;
use crate::{PositionComponent, ExtractSource, SpriteComponent};

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

#[derive(Default)]
struct SpriteExtractJobImpl {
    per_frame_data: Vec<Vec3>,
    per_view_data: Vec<Vec<Vec3>>,
}

impl DefaultExtractJobImpl<ExtractSource> for SpriteExtractJobImpl {
    fn extract_begin(
        &mut self,
        _source: &ExtractSource,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) {
        log::debug!("extract_begin {}", self.feature_debug_name());
        self.per_frame_data
            .reserve(frame_packet.frame_node_count(self.feature_index()));

        self.per_view_data.reserve(views.len());
        for view in views {
            self.per_view_data.push(Vec::with_capacity(
                frame_packet.view_node_count(view, self.feature_index()),
            ));
        }
    }

    fn extract_frame_node(
        &mut self,
        source: &ExtractSource,
        frame_node: PerFrameNode,
        frame_node_index: u32,
    ) {
        log::debug!(
            "extract_frame_node {} {}",
            self.feature_debug_name(),
            frame_node_index
        );

        let render_node_index = frame_node.render_node_index();
        let render_node_handle = RawSlabKey::<SpriteRenderNode>::new(render_node_index);

        let sprite_nodes = source.resources.get::<SpriteRenderNodeSet>().unwrap();
        let sprite_render_node = sprite_nodes.sprites.get(render_node_handle).unwrap();
        let position_component = source
            .world
            .get_component::<PositionComponent>(sprite_render_node.entity)
            .unwrap();

        self.per_frame_data.push(position_component.position);
    }

    fn extract_view_node(
        &mut self,
        _source: &ExtractSource,
        view: &RenderView,
        view_node: PerViewNode,
        view_node_index: u32,
    ) {
        log::debug!(
            "extract_view_nodes {} {} {}",
            self.feature_debug_name(),
            view_node_index,
            self.per_frame_data[view_node.frame_node_index() as usize]
        );
        let frame_data = self.per_frame_data[view_node.frame_node_index() as usize];
        self.per_view_data[view.view_index()].push(frame_data);
    }

    fn extract_view_finalize(
        &mut self,
        _source: &ExtractSource,
        _view: &RenderView,
    ) {
        log::debug!("extract_view_finalize {}", self.feature_debug_name());
    }

    fn extract_frame_finalize(
        self,
        _source: &ExtractSource,
    ) -> Box<dyn PrepareJob> {
        log::debug!("extract_frame_finalize {}", self.feature_debug_name());

        let prepare_impl = SpritePrepareJobImpl {
            per_frame_data: self.per_frame_data,
            per_view_data: self.per_view_data
        };

        Box::new(DefaultPrepareJob::new(prepare_impl))

    }

    fn feature_debug_name(&self) -> &'static str {
        SpriteRenderFeature::feature_debug_name()
    }
    fn feature_index(&self) -> RenderFeatureIndex {
        SpriteRenderFeature::feature_index()
    }
}

pub fn create_sprite_extract_job() -> Box<dyn ExtractJob<ExtractSource>> {
    Box::new(DefaultExtractJob::new(SpriteExtractJobImpl::default()))
}

// pub struct SpriteExtractJob {
//     inner: Box<DefaultExtractJob<ExtractSource, SpriteExtractJobImpl>>,
// }
//
// impl SpriteExtractJob {
//     pub fn new() -> Self {
//         let job_impl = SpriteExtractJobImpl::default();
//
//         SpriteExtractJob {
//             inner: Box::new(DefaultExtractJob::new(job_impl)),
//         }
//     }
// }
//
// impl ExtractJob<ExtractSource> for SpriteExtractJob {
//     fn extract(
//         self: Box<Self>,
//         source: &ExtractSource,
//         frame_packet: &FramePacket,
//         views: &[&RenderView],
//     ) -> Box<dyn PrepareJob> {
//         self.inner.extract(source, frame_packet, views)
//     }
//
//     fn feature_debug_name(&self) -> &'static str {
//         self.inner.feature_debug_name()
//     }
// }

struct SpritePrepareJobImpl {
    per_frame_data: Vec<Vec3>,
    per_view_data: Vec<Vec<Vec3>>,
}

impl DefaultPrepareJobImpl for SpritePrepareJobImpl {
    fn prepare_begin(&mut self, frame_packet: &FramePacket, views: &[&RenderView]) {
        log::debug!("prepare_begin {}", self.feature_debug_name());

    }

    fn prepare_frame_node(&mut self, frame_node: PerFrameNode, frame_node_index: u32) {
        log::debug!(
            "prepare_frame_node {} {}",
            self.feature_debug_name(),
            frame_node_index
        );
    }

    fn prepare_view_node(&mut self, view: &RenderView, view_node: PerViewNode, view_node_index: u32) {
        log::debug!(
            "prepare_view_node {} {} {}",
            self.feature_debug_name(),
            view_node_index,
            self.per_frame_data[view_node.frame_node_index() as usize]
        );
    }

    fn prepare_view_finalize(&mut self, view: &RenderView) {
        log::debug!("prepare_view_finalize {}", self.feature_debug_name());
    }

    fn prepare_frame_finalize(self) {
        log::debug!("prepare_frame_finalize {}", self.feature_debug_name());
    }

    fn feature_debug_name(&self) -> &'static str {
        SpriteRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> u32 {
        SpriteRenderFeature::feature_index()
    }
}

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

pub struct SpriteRenderNodeSet {
    sprites: RawSlab<SpriteRenderNode>,
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

    fn max_render_node_count(&self) -> usize {
        self.sprites.storage_size()
    }
}
