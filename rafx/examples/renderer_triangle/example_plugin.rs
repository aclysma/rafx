use crate::phases::OpaqueRenderPhase;
use rafx::render_features::RenderRegistryBuilder;
use rafx::renderer::RendererAssetPlugin;

pub struct ExampleRendererPlugin;

impl RendererAssetPlugin for ExampleRendererPlugin {
    fn configure_render_registry(
        &self,
        render_registry_builder: RenderRegistryBuilder,
    ) -> RenderRegistryBuilder {
        render_registry_builder.register_render_phase::<OpaqueRenderPhase>("Opaque")
    }
}
