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

        pub struct $struct_name;

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
    };
}
