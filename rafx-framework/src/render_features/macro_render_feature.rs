/// Use to declare a new render feature that can be registered. Registration allows easy global
/// access to the render feature index from anywhere in the binary
///
/// Use like this:
///      rafx::declare_render_feature!(Debug3DRenderFeature, DEBUG_3D_RENDER_FEATURE);
///
/// The first name is all that really matters, the second name just needs to be a constant that is
/// exposed via the first name (i.e. Debug3DRenderFeature::feature_index())
///
/// This macro will also define the following helper functions in the same scope.
/// - `render_feature_index()`: Syntactic sugar for Debug3DRenderFeature::feature_index().
/// - `render_feature_debug_name()`: Syntactic sugar for Debug3DRenderFeature::feature_debug_name().
/// - `render_feature_debug_constants()`: Returns a struct containing `&'static str` debug strings for the feature.
#[macro_export]
macro_rules! declare_render_feature {
    ($struct_name:ident, $atomic_constant_name:ident) => {
        static $atomic_constant_name: std::sync::atomic::AtomicI32 =
            std::sync::atomic::AtomicI32::new(-1);

        pub struct $struct_name;

        static RENDER_FEATURE_DEBUG_CONSTANTS: RenderFeatureDebugConstants = RenderFeatureDebugConstants {
            feature_name: stringify!($struct_name),

            begin_per_frame_extract: stringify!($struct_name begin_per_frame_extract),
            extract_render_object_instance: stringify!($struct_name extract_render_object_instance),
            extract_render_object_instance_per_view: stringify!($struct_name extract_render_object_instance_per_view),
            end_per_view_extract: stringify!($struct_name end_per_view_extract),
            end_per_frame_extract: stringify!($struct_name end_per_frame_extract),

            begin_per_frame_prepare: stringify!($struct_name begin_per_frame_prepare),
            prepare_render_object_instance: stringify!($struct_name prepare_render_object_instance),
            prepare_render_object_instance_per_view: stringify!($struct_name prepare_render_object_instance_per_view),
            end_per_view_prepare: stringify!($struct_name end_per_view_prepare),
            end_per_frame_prepare: stringify!($struct_name end_per_frame_prepare),

            on_begin_execute_graph: stringify!($struct_name on_begin_execute_graph),
            render_submit_node: stringify!($struct_name render_submit_node),
            apply_setup: stringify!($struct_name apply_setup),
            revert_setup: stringify!($struct_name revert_setup),
        };

        impl RenderFeature for $struct_name {
            fn set_feature_index(index: RenderFeatureIndex) {
                assert_eq!(
                    $struct_name::feature_index(),
                    RenderFeatureIndex::MAX,
                    "feature {} was already registered",
                    $struct_name::feature_debug_name(),
                );

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
                render_feature_debug_name()
            }

            fn feature_debug_constants() -> &'static RenderFeatureDebugConstants {
                render_feature_debug_constants()
            }
        }

        #[inline(always)]
        fn render_feature_index() -> RenderFeatureIndex {
            $struct_name::feature_index()
        }

        #[inline(always)]
        fn render_feature_debug_name() -> &'static str {
            render_feature_debug_constants().feature_name
        }

        #[inline(always)]
        fn render_feature_debug_constants() -> &'static RenderFeatureDebugConstants {
            &RENDER_FEATURE_DEBUG_CONSTANTS
        }
    };
}
