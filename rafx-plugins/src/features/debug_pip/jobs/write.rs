use rafx::render_feature_write_job_prelude::*;

use super::*;
use crate::shaders::debug_pip::debug_pip_frag;
use rafx::api::RafxPrimitiveTopology;
use rafx::framework::{MaterialPassResource, ResourceArc, VertexDataSetLayout};
use rafx::render_features::RenderSubmitNodeArgs;
use rafx::renderer::SwapchainRenderResource;
use std::marker::PhantomData;

lazy_static::lazy_static! {
    pub static ref EMPTY_VERTEX_LAYOUT : VertexDataSetLayout = {
        VertexDataSetLayout::new(vec![], RafxPrimitiveTopology::TriangleList)
    };
}

pub struct DebugPipWriteJob<'write> {
    debug_pip_material_pass: Option<ResourceArc<MaterialPassResource>>,
    frame_packet: Box<DebugPipFramePacket>,
    _submit_packet: Box<DebugPipSubmitPacket>,
    phantom: PhantomData<&'write ()>,
}

impl<'write> DebugPipWriteJob<'write> {
    pub fn new(
        _write_context: &RenderJobWriteContext<'write>,
        frame_packet: Box<DebugPipFramePacket>,
        submit_packet: Box<DebugPipSubmitPacket>,
    ) -> Arc<dyn RenderFeatureWriteJob<'write> + 'write> {
        Arc::new(Self {
            debug_pip_material_pass: {
                frame_packet
                    .per_frame_data()
                    .get()
                    .debug_pip_material_pass
                    .clone()
            },
            frame_packet,
            _submit_packet: submit_packet,
            phantom: Default::default(),
        })
    }
}

impl<'write> RenderFeatureWriteJob<'write> for DebugPipWriteJob<'write> {
    fn view_frame_index(
        &self,
        view: &RenderView,
    ) -> ViewFrameIndex {
        self.frame_packet.view_frame_index(view)
    }

    fn render_submit_node(
        &self,
        write_context: &mut RenderJobCommandBufferContext,
        args: RenderSubmitNodeArgs,
    ) -> RafxResult<()> {
        profiling::scope!(super::render_feature_debug_constants().render_submit_node);

        if let Some(debug_pip_material_pass) = &self.debug_pip_material_pass {
            //
            // Create descriptor sets for rendering debug images. This is normally handled in prepare,
            // but since we're drawing render targets, they may not be in a bindable state during the
            // prepare job.
            //
            let descriptor_set_layouts = debug_pip_material_pass.get_raw().descriptor_set_layouts;
            let debug_pip_render_resource = write_context
                .graph_context
                .render_resources()
                .fetch::<DebugPipRenderResource>();
            let swapchain_resource = write_context
                .graph_context
                .render_resources()
                .fetch::<SwapchainRenderResource>();
            let swapchain_size = swapchain_resource
                .surface_info()
                .unwrap()
                .swapchain_surface_info
                .extents;

            let mut descriptor_set_allocator = write_context
                .resource_context
                .create_descriptor_set_allocator();
            let mut descriptor_sets =
                Vec::with_capacity(debug_pip_render_resource.sampled_render_graph_images.len());
            for image_usage in &debug_pip_render_resource.sampled_render_graph_images {
                let image = write_context
                    .graph_context
                    .image_view(*image_usage)
                    .unwrap();
                let descriptor_set = descriptor_set_allocator.create_descriptor_set_with_writer(
                    &descriptor_set_layouts[debug_pip_frag::DEBUG_PIP_TEX_DESCRIPTOR_SET_INDEX],
                    debug_pip_frag::DescriptorSet0Args {
                        debug_pip_tex: &image,
                    },
                )?;

                descriptor_sets.push(descriptor_set);
            }

            descriptor_set_allocator.flush_changes()?;

            //
            // Now we draw everything
            //
            let command_buffer = &write_context.command_buffer;

            let pipeline = write_context
                .resource_context
                .graphics_pipeline_cache()
                .get_or_create_graphics_pipeline(
                    Some(args.render_phase_index),
                    &debug_pip_material_pass,
                    &write_context.render_target_meta,
                    &EMPTY_VERTEX_LAYOUT,
                )?;

            command_buffer.cmd_bind_pipeline(&*pipeline.get_raw().pipeline)?;

            let image_width = 400.0;
            let image_height = 400.0;

            let x = swapchain_size.width as f32 - image_width;
            let y = swapchain_size.height as f32 - image_height;

            for descriptor_set in descriptor_sets {
                descriptor_set.bind(command_buffer)?;
                command_buffer.cmd_set_viewport(x, y, image_width, image_height, 0.0, 1.0)?;
                command_buffer.cmd_draw(3, 0)?;
            }
        }

        Ok(())
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        super::render_feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }
}
