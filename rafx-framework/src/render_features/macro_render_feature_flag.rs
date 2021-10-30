/// Use to declare a new render feature flag that can be registered. Registration allows easy global
/// access to the render feature flag index from anywhere in the binary
///
/// Each RenderView contains a bitmask of supported feature flags. The supported feature flags can
/// be queried or changed at run-time allowing features to provide custom behavior on a per-view
/// basis depending on the state of the RenderView's feature flags.
///
/// Use like this:
///      rafx::declare_render_feature_flag!(MeshUnlitRenderFeatureFlag, MESH_UNLIT_FLAG_INDEX);
///
/// The first name is all that really matters, the second name just needs to be a constant that is
/// exposed via the first name (i.e. MeshUnlitRenderFeatureFlag::feature_flag_index())
#[macro_export]
macro_rules! declare_render_feature_flag {
    ($struct_name:ident, $atomic_constant_name:ident) => {
        static $atomic_constant_name: std::sync::atomic::AtomicI32 =
            std::sync::atomic::AtomicI32::new(-1);

        pub struct $struct_name;

        impl $crate::render_features::RenderFeatureFlag for $struct_name {
            fn set_feature_flag_index(index: $crate::render_features::RenderFeatureFlagIndex) {
                assert_eq!(
                    $struct_name::feature_flag_index(),
                    $crate::render_features::RenderFeatureFlagIndex::MAX,
                    "feature flag {} was already registered",
                    $struct_name::feature_flag_debug_name(),
                );

                $atomic_constant_name.store(
                    index.try_into().unwrap(),
                    std::sync::atomic::Ordering::Release,
                );
            }

            fn feature_flag_index() -> $crate::render_features::RenderFeatureFlagIndex {
                $atomic_constant_name.load(std::sync::atomic::Ordering::Acquire)
                    as $crate::render_features::RenderFeatureFlagIndex
            }

            fn feature_flag_debug_name() -> &'static str {
                stringify!($struct_name)
            }
        }
    };
}
