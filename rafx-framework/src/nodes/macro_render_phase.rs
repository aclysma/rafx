// Use to declare a new render phase that can be registered. Registration allows easy global
// access to the render phase index from anywhere in the binary
//
// Use like this:
//      rafx::declare_render_phase!(Debug3dRenderFeature, DEBUG_3D_RENDER_FEATURE, sort_fn);
//
// The first name is all that really matters, the second name just needs to be a constant that is
// exposed via the first name (i.e. Debug3dRenderFeature::feature_index())
//
// The function provided is a sort function like this:
//
// fn sort_submit_nodes(mut submit_nodes: Vec<SubmitNode>) -> Vec<SubmitNode> {
//     // Sort by feature
//     log::trace!("Sort phase {}", MyRenderPhase::render_phase_debug_name());
//     submit_nodes.sort_unstable_by(|a, b| a.feature_index().cmp(&b.feature_index()));
//
//     submit_nodes
// }
//
// This can be use to implement back to front and front to back sorting, or just sort by feature
// if order doesn't matter to get the best batching
#[macro_export]
macro_rules! declare_render_phase {
    ($struct_name:ident, $atomic_constant_name:ident, $sort_fn:ident) => {
        static $atomic_constant_name: std::sync::atomic::AtomicI32 =
            std::sync::atomic::AtomicI32::new(-1);

        pub struct $struct_name;

        impl RenderPhase for $struct_name {
            fn set_render_phase_index(index: RenderPhaseIndex) {
                use std::convert::TryInto;
                $atomic_constant_name.store(
                    index.try_into().unwrap(),
                    std::sync::atomic::Ordering::Release,
                );
            }

            fn render_phase_index() -> RenderPhaseIndex {
                $atomic_constant_name.load(std::sync::atomic::Ordering::Acquire) as RenderPhaseIndex
            }

            fn sort_submit_nodes(submit_nodes: Vec<SubmitNode>) -> Vec<SubmitNode> {
                $sort_fn(submit_nodes)
            }

            fn render_phase_debug_name() -> &'static str {
                stringify!($struct_name)
            }
        }
    };
}
