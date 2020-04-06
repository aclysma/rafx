
pub trait RenderFeatureImpl {
    fn extract(&self);
    fn prepare(&self);
    fn submit(&self);
}

trait RenderFeature {
    fn set_feature_index(index: i32);
    fn feature_index() -> i32;
    //fn create_impl() -> Box<dyn RenderFeatureImpl>;
}

static RENDER_REGISTRY_FEATURE_COUNT : AtomicI32 = AtomicI32::new(0);

struct RenderRegistry {
    //feature_create_cb: Vec<Box<Fn() -> Box<dyn RenderFeatureImpl>>>,
    //registered_feature_count: i32
}

trait RenderFeatureImplCreator {
    fn create() -> Box<dyn RenderFeatureImpl>;
}

struct Renderer {
    feature_impls: Vec<Box<dyn RenderFeatureImpl>>
}

impl RenderRegistry {
    fn register_feature<T>(/*&mut self, feature: T*/) where T: RenderFeature {
        let feature_index = RENDER_REGISTRY_FEATURE_COUNT.fetch_add(1, Ordering::AcqRel);
        T::set_feature_index(feature_index);


        //self.registered_feature_count += 1;

        //let create_cb = Box::new(|| T::create_impl());
        //self.feature_create_cb.push(create_cb);
    }

    // fn create_renderer(&self) -> Renderer {
    //     let feature_impls : Vec<_> = self.feature_create_cb.iter().map(|cb| (cb)()).collect();
    //     Renderer {
    //         feature_impls
    //     }
    // }
}










use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering;

use renderer_base::slab::RawSlab;
use crate::render_nodes::*;
use crate::frame_packet::FramePacket;

static SPRITE_FEATURE_INDEX : AtomicI32 = AtomicI32::new(-1);

struct SpriteFeature {

}

impl RenderFeature for SpriteFeature {
    //type Impl = SpriteRenderFeatureImpl;

    fn set_feature_index(index: i32) {
        SPRITE_FEATURE_INDEX.store(index, Ordering::Release);
    }

    fn feature_index() -> i32 {
        SPRITE_FEATURE_INDEX.load(Ordering::Acquire)
    }

    // fn create_impl() -> Box<dyn RenderFeatureImpl> {
    //     Box::new(SpriteRenderFeatureImpl::default())
    // }
}

#[derive(Default)]
struct SpriteRenderFeatureImpl {

}

impl RenderFeatureImpl for SpriteRenderFeatureImpl {
    fn extract(&self) {
        println!("extract");
    }

    fn prepare(&self) {
        println!("prepare");
    }

    fn submit(&self) {
        println!("submit");
    }
}

#[derive(Default)]
struct SpriteRenderNodeSet {
    sprites: RawSlab<SpriteRenderNode>,
}

impl SpriteRenderNodeSet {
    pub fn register_sprite(&mut self, node: SpriteRenderNode) -> SpriteRenderNodeHandle {
        //TODO: Request streaming in a resource
        SpriteRenderNodeHandle(self.sprites.allocate(node))
    }

    pub fn unregister_sprite(&mut self, handle: SpriteRenderNodeHandle) {
        //TODO: Decrement reference count for resource
        self.sprites.free(&handle.0);
    }
}



fn test() {
    //let mut registry = RenderRegistry::new();
    RenderRegistry::register_feature::<SpriteRenderFeature>();

}