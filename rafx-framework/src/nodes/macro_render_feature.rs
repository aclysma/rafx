// Use to declare a new render feature that can be registered. Registration allows easy global
// access to the render feature index from anywhere in the binary
//
// Use like this:
//      rafx::declare_render_feature!(Debug3dRenderFeature, DEBUG_3D_RENDER_FEATURE);
//
// The first name is all that really matters, the second name just needs to be a constant that is
// exposed via the first name (i.e. Debug3dRenderFeature::feature_index())
#[macro_export]
macro_rules! declare_render_feature {
    ($struct_name:ident, $atomic_constant_name:ident) => {
        static $atomic_constant_name: std::sync::atomic::AtomicI32 =
            std::sync::atomic::AtomicI32::new(-1);

        #[rustfmt::skip]
        static extract_scope: &'static str = stringify!($struct_name extract);

        #[rustfmt::skip]
        static prepare_scope: &'static str = stringify!($struct_name prepare);

        #[rustfmt::skip]
        static on_begin_execute_graph_scope: &'static str = stringify!($struct_name on_begin_execute_graph);
        static render_element_scope: &'static str = stringify!($struct_name render_element);
        static apply_setup_scope: &'static str = stringify!($struct_name apply_setup);
        static revert_setup_scope: &'static str = stringify!($struct_name revert_setup);

        pub struct $struct_name;
        type RenderFeatureType = $struct_name;

        impl RenderFeature for $struct_name {
            fn set_feature_index(index: RenderFeatureIndex) {
                $atomic_constant_name.store(
                    index.try_into().unwrap(),
                    std::sync::atomic::Ordering::Release,
                );
            }

            fn feature_index() -> RenderFeatureIndex {
                $atomic_constant_name.load(std::sync::atomic::Ordering::Acquire)
                    as RenderFeatureIndex
            }

            fn feature_debug_name() -> &'static str {
                stringify!($struct_name)
            }
        }

        fn render_feature_index() -> RenderFeatureIndex {
            $struct_name::feature_index()
        }

        fn render_feature_debug_name() -> &'static str {
            $struct_name::feature_debug_name()
        }
    };
}

#[macro_export]
macro_rules! declare_render_feature_mod {
    () => {
        use rafx::nodes::{RenderFeature, RenderFeatureIndex};
        use std::convert::TryInto;
    };
}

#[macro_export]
macro_rules! declare_render_feature_renderer_plugin {
    () => {
        use extract::ExtractJobImpl;
        use rafx::api::extra::upload::RafxTransferUpload;
        use rafx::api::RafxResult;
        use rafx::assets::distill_impl::AssetResource;
        use rafx::assets::AssetManager;
        use rafx::base::resource_map::ResourceMap;
        use rafx::framework::RenderResources;
        use rafx::nodes::{ExtractJob, ExtractResources, RenderRegistryBuilder};
        use rafx::renderer::RendererPlugin;
    };
}

#[macro_export]
macro_rules! declare_render_feature_extract_job {
    () => {
        use super::prepare::PrepareJobImpl;
        use super::{extract_scope, render_feature_debug_name, render_feature_index};
        use rafx::nodes::{
            ExtractJob, FramePacket, PrepareJob, RenderFeatureIndex, RenderJobExtractContext,
            RenderView,
        };
    };
}

#[macro_export]
macro_rules! declare_render_feature_prepare_job {
    () => {
        use super::write::FeatureCommandWriterImpl;
        use super::{prepare_scope, render_feature_debug_name, render_feature_index};
        use rafx::nodes::{
            FeatureCommandWriter, FeatureSubmitNodes, FramePacket, PrepareJob, RenderFeatureIndex,
            RenderJobPrepareContext, RenderView, ViewSubmitNodes,
        };
    };
}

#[macro_export]
macro_rules! declare_render_feature_write_job {
    () => {
        use super::{
            apply_setup_scope, on_begin_execute_graph_scope, render_element_scope,
            render_feature_debug_name, render_feature_index, revert_setup_scope,
        };
        use rafx::api::RafxResult;
        use rafx::nodes::{
            FeatureCommandWriter, RenderFeatureIndex, RenderJobWriteContext, RenderPhaseIndex,
            RenderView, SubmitNodeId,
        };
    };
}
