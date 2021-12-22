use rafx::graph::RenderGraphImageUsageId;

//TODO: We don't use this right now but leaving it in for now to hook up more control for this
// from the main thread (i.e. let debug UI control this)
#[derive(Default)]
pub struct DebugPipResource {
    //pub(super) debug_pip_textures: Vec<ResourceArc<ImageViewResource>>,
}

impl DebugPipResource {
    // pub fn debug_pip_textures(&self) -> &Vec<ResourceArc<ImageViewResource>> {
    //     &self.debug_pip_textures
    // }
    //
    // pub fn debug_pip_texture_mut(&mut self) -> &mut Vec<ResourceArc<ImageViewResource>> {
    //     &mut self.debug_pip_textures
    // }
}

#[derive(Default)]
pub struct DebugPipRenderResource {
    pub(super) render_graph_images: Vec<RenderGraphImageUsageId>,
    pub(super) sampled_render_graph_images: Vec<RenderGraphImageUsageId>,
}

impl DebugPipRenderResource {
    pub fn add_render_graph_image(
        &mut self,
        image: RenderGraphImageUsageId,
    ) {
        self.render_graph_images.push(image);
    }

    pub fn render_graph_images(&self) -> &[RenderGraphImageUsageId] {
        &self.render_graph_images
    }

    pub fn set_sampled_render_graph_images(
        &mut self,
        sampled_render_graph_images: Vec<RenderGraphImageUsageId>,
    ) {
        self.sampled_render_graph_images = sampled_render_graph_images;
    }

    pub fn clear(&mut self) {
        self.render_graph_images.clear();
    }
}
