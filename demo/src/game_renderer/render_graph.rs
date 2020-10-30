use ash::vk;
use renderer::assets::{vk_description as dsc, ResourceContext};
use renderer::assets::graph::*;
use crate::VkDeviceContext;
use ash::prelude::VkResult;
use renderer::assets::resources::{
    ResourceArc, ImageViewResource, RenderPassResource, MaterialPassResource,
};
use crate::render_contexts::RenderJobWriteContext;
use renderer::nodes::{PreparedRenderData, RenderView};
use crate::phases::{OpaqueRenderPhase, UiRenderPhase};
use renderer::vulkan::SwapchainInfo;
use ash::version::DeviceV1_0;

pub struct BuildRenderGraphResult {
    pub ui_renderpass: Option<ResourceArc<RenderPassResource>>,
    pub executor: RenderGraphExecutor<RenderGraphUserContext>,
}

// Any data you want available within rendergraph execution callbacks should go here. This can
// include data that is not known until later after the extract/prepare phases have completed.
pub struct RenderGraphUserContext {
    pub prepared_render_data: Box<PreparedRenderData<RenderJobWriteContext>>,
}

pub fn build_render_graph(
    device_context: &VkDeviceContext,
    resource_context: &ResourceContext,
    swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    swapchain_info: &SwapchainInfo,
    swapchain_image: ResourceArc<ImageViewResource>,
    main_view: RenderView,
    bloom_extract_material_pass: ResourceArc<MaterialPassResource>,
    bloom_blur_material_pass: ResourceArc<MaterialPassResource>,
    bloom_combine_material_pass: ResourceArc<MaterialPassResource>,
) -> VkResult<BuildRenderGraphResult> {
    //TODO: Fix this back to be color format - need to happen in the combine pass
    let color_format = swapchain_surface_info.color_format;
    let depth_format = swapchain_surface_info.depth_format;
    let swapchain_format = swapchain_surface_info.surface_format.format;
    let samples = swapchain_surface_info.msaa_level.into();

    let mut graph = RenderGraphBuilder::default();
    let mut graph_callbacks = RenderGraphNodeCallbacks::<RenderGraphUserContext>::default();

    let opaque_pass = {
        struct OpaquePass {
            node: RenderGraphNodeId,
            color: RenderGraphImageUsageId,
            depth: RenderGraphImageUsageId,
        }

        let node = graph.add_node("Opaque", RenderGraphQueue::DefaultGraphics);

        let color = graph.create_color_attachment(
            node,
            0,
            Some(vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            }),
            RenderGraphImageConstraint {
                samples: Some(samples),
                format: Some(color_format),
                ..Default::default()
            },
        );
        graph.set_image_name(color, "color");

        let depth = graph.create_depth_attachment(
            node,
            Some(vk::ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            }),
            RenderGraphImageConstraint {
                samples: Some(samples),
                format: Some(depth_format),
                ..Default::default()
            },
        );
        graph.set_image_name(depth, "depth");

        graph_callbacks.add_renderphase_dependency::<OpaqueRenderPhase>(node);

        let main_view = main_view.clone();
        graph_callbacks.set_renderpass_callback(node, move |args, user_context| {
            let mut write_context = RenderJobWriteContext::from_graph_visit_render_pass_args(&args);
            user_context
                .prepared_render_data
                .write_view_phase::<OpaqueRenderPhase>(&main_view, &mut write_context);
            Ok(())
        });

        OpaquePass { node, color, depth }
    };

    let bloom_extract_pass = {
        struct BloomExtractPass {
            node: RenderGraphNodeId,
            sdr_image: RenderGraphImageUsageId,
            hdr_image: RenderGraphImageUsageId,
        }

        let node = graph.add_node("BloomExtract", RenderGraphQueue::DefaultGraphics);

        let sdr_image = graph.create_color_attachment(
            node,
            0,
            Default::default(),
            RenderGraphImageConstraint {
                samples: Some(vk::SampleCountFlags::TYPE_1),
                format: Some(color_format),
                ..Default::default()
            },
        );
        graph.set_image_name(sdr_image, "sdr");
        let hdr_image = graph.create_color_attachment(
            node,
            1,
            Some(vk::ClearColorValue::default()),
            RenderGraphImageConstraint {
                samples: Some(vk::SampleCountFlags::TYPE_1),
                format: Some(color_format),
                ..Default::default()
            },
        );
        graph.set_image_name(hdr_image, "hdr");

        let sample_image = graph.sample_image(
            node,
            opaque_pass.color,
            RenderGraphImageConstraint {
                samples: Some(vk::SampleCountFlags::TYPE_1),
                ..Default::default()
            },
        );

        graph_callbacks.set_renderpass_callback(node, move |args, _user_context| {
            // Get the color image from before
            let sample_image = args.graph_context.image(sample_image).unwrap();

            // Get the pipeline
            let pipeline = args
                .graph_context
                .resource_context()
                .graphics_pipeline_cache()
                .get_or_create_graphics_pipeline(&bloom_extract_material_pass, args.renderpass)?;

            // Set up a descriptor set pointing at the image so we can sample from it
            let mut descriptor_set_allocator = args
                .graph_context
                .resource_context()
                .create_descriptor_set_allocator();
            let mut bloom_extract_material_dyn_set = descriptor_set_allocator
                .create_dyn_descriptor_set_uninitialized(
                    &pipeline.get_raw().pipeline_layout.get_raw().descriptor_sets[0],
                )?;
            bloom_extract_material_dyn_set.set_image_raw(0, sample_image.get_raw().image_view);
            bloom_extract_material_dyn_set.flush(&mut descriptor_set_allocator)?;

            // Flush the descriptor set change
            descriptor_set_allocator.flush_changes()?;

            // Draw calls
            let command_buffer = args.command_buffer;
            let device = args.graph_context.device_context().device();
            unsafe {
                device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.get_raw().pipelines[0],
                );

                device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.get_raw().pipeline_layout.get_raw().pipeline_layout,
                    0,
                    &[bloom_extract_material_dyn_set.descriptor_set().get()],
                    &[],
                );

                device.cmd_draw(command_buffer, 3, 1, 0, 0);
            }

            Ok(())
        });

        BloomExtractPass {
            node,
            sdr_image,
            hdr_image,
        }
    };

    let bloom_blur_pass = {
        struct BloomBlurPass {
            color: RenderGraphImageUsageId,
        }

        let mut blur_src = bloom_extract_pass.hdr_image;
        let mut blur_dst = None;

        for blur_pass_index in 0..10 {
            let node = graph.add_node("BloomBlur", RenderGraphQueue::DefaultGraphics);
            blur_dst = Some(graph.create_color_attachment(
                node,
                0,
                Some(Default::default()),
                RenderGraphImageConstraint {
                    samples: Some(vk::SampleCountFlags::TYPE_1),
                    format: Some(color_format),
                    ..Default::default()
                },
            ));
            graph.set_image_name(blur_dst.unwrap(), "blur_dst");

            let sample_image = graph.sample_image(node, blur_src, Default::default());
            graph.set_image_name(blur_src, "blur_src");

            let bloom_blur_material_pass = bloom_blur_material_pass.clone();
            graph_callbacks.set_renderpass_callback(node, move |args, _user_context| {
                // Get the color image from before
                let sample_image = args.graph_context.image(sample_image).unwrap();

                // Get the pipeline
                let pipeline = args
                    .graph_context
                    .resource_context()
                    .graphics_pipeline_cache()
                    .get_or_create_graphics_pipeline(&bloom_blur_material_pass, args.renderpass)?;

                // Set up a descriptor set pointing at the image so we can sample from it
                let mut descriptor_set_allocator = args
                    .graph_context
                    .resource_context()
                    .create_descriptor_set_allocator();
                let mut bloom_extract_material_dyn_set = descriptor_set_allocator
                    .create_dyn_descriptor_set_uninitialized(
                        &pipeline.get_raw().pipeline_layout.get_raw().descriptor_sets[0],
                    )?;
                bloom_extract_material_dyn_set.set_image_raw(0, sample_image.get_raw().image_view);

                let mut bloom_blur_material_dyn_set = descriptor_set_allocator
                    .create_dyn_descriptor_set_uninitialized(
                        &pipeline.get_raw().pipeline_layout.get_raw().descriptor_sets[0],
                    )?;
                bloom_blur_material_dyn_set.set_image(0, sample_image);

                let blur_horizontally: u32 = blur_pass_index % 2;

                bloom_blur_material_dyn_set.set_buffer_data(2, &blur_horizontally);
                bloom_blur_material_dyn_set.flush(&mut descriptor_set_allocator)?;
                descriptor_set_allocator.flush_changes()?;

                // Draw calls
                let command_buffer = args.command_buffer;
                let device = args.graph_context.device_context().device();
                unsafe {
                    device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline.get_raw().pipelines[0],
                    );

                    device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline.get_raw().pipeline_layout.get_raw().pipeline_layout,
                        0,
                        &[bloom_blur_material_dyn_set.descriptor_set().get()],
                        &[],
                    );

                    device.cmd_draw(command_buffer, 3, 1, 0, 0);
                }

                Ok(())
            });

            blur_src = blur_dst.unwrap();
        }

        BloomBlurPass {
            color: blur_dst.unwrap(),
        }
    };

    let bloom_combine_pass = {
        struct BloomCombinePass {
            node: RenderGraphNodeId,
            color: RenderGraphImageUsageId,
        }

        let node = graph.add_node("BloomCombine", RenderGraphQueue::DefaultGraphics);

        let color = graph.create_color_attachment(
            node,
            0,
            Default::default(),
            RenderGraphImageConstraint {
                format: Some(swapchain_format),
                ..Default::default()
            },
        );
        graph.set_image_name(color, "color");

        let sdr_image = graph.sample_image(node, bloom_extract_pass.sdr_image, Default::default());
        graph.set_image_name(sdr_image, "sdr");

        let hdr_image = graph.sample_image(node, bloom_blur_pass.color, Default::default());
        graph.set_image_name(hdr_image, "hdr");

        graph_callbacks.set_renderpass_callback(node, move |args, _user_context| {
            // Get the color image from before
            let sdr_image = args.graph_context.image(sdr_image).unwrap();
            let hdr_image = args.graph_context.image(hdr_image).unwrap();

            // Get the pipeline
            let pipeline = args
                .graph_context
                .resource_context()
                .graphics_pipeline_cache()
                .get_or_create_graphics_pipeline(&bloom_combine_material_pass, args.renderpass)?;

            // Set up a descriptor set pointing at the image so we can sample from it
            let mut descriptor_set_allocator = args
                .graph_context
                .resource_context()
                .create_descriptor_set_allocator();
            let mut bloom_combine_material_dyn_set = descriptor_set_allocator
                .create_dyn_descriptor_set_uninitialized(
                    &pipeline.get_raw().pipeline_layout.get_raw().descriptor_sets[0],
                )?;
            bloom_combine_material_dyn_set.set_image(0, sdr_image);
            bloom_combine_material_dyn_set.set_image(1, hdr_image);
            bloom_combine_material_dyn_set.flush(&mut descriptor_set_allocator)?;
            descriptor_set_allocator.flush_changes()?;

            // Draw calls
            let command_buffer = args.command_buffer;
            let device = args.graph_context.device_context().device();

            unsafe {
                device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.get_raw().pipelines[0],
                );

                device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.get_raw().pipeline_layout.get_raw().pipeline_layout,
                    0,
                    &[bloom_combine_material_dyn_set.descriptor_set().get()],
                    &[],
                );

                device.cmd_draw(command_buffer, 3, 1, 0, 0);
            }

            Ok(())
        });

        BloomCombinePass { node, color }
    };

    let ui_pass = {
        struct UiPass {
            node: RenderGraphNodeId,
            color: RenderGraphImageUsageId,
        }

        // This node has a single color attachment
        let node = graph.add_node("Ui", RenderGraphQueue::DefaultGraphics);
        let color =
            graph.modify_color_attachment(node, bloom_combine_pass.color, 0, Default::default());
        graph.set_image_name(color, "color");

        // Adding a phase dependency insures that we create all the pipelines for materials
        // associated with the phase. This controls how long we keep the pipelines allocated and
        // allows us to precache pipelines for materials as they are loaded
        graph_callbacks.add_renderphase_dependency::<UiRenderPhase>(node);

        // When the node is executed, we automatically set up the renderpass/framebuffer/command
        // buffer. Just add the draw calls.
        let main_view = main_view.clone();
        graph_callbacks.set_renderpass_callback(node, move |args, user_context| {
            // Kick the material system to emit all draw calls for the UiRenderPhase for the view
            let mut write_context = RenderJobWriteContext::from_graph_visit_render_pass_args(&args);
            user_context
                .prepared_render_data
                .write_view_phase::<UiRenderPhase>(&main_view, &mut write_context);
            Ok(())
        });

        UiPass { node, color }
    };

    let _swapchain_output_image_id = graph.set_output_image(
        ui_pass.color,
        swapchain_image,
        RenderGraphImageSpecification {
            samples: vk::SampleCountFlags::TYPE_1,
            format: swapchain_format,
            aspect_flags: vk::ImageAspectFlags::COLOR,
            usage_flags: swapchain_info.image_usage_flags,
        },
        dsc::ImageLayout::PresentSrcKhr,
        vk::AccessFlags::empty(),
        vk::PipelineStageFlags::empty(),
        vk::ImageAspectFlags::COLOR,
    );

    //
    // Create the executor, it needs to have access to the resource manager to add framebuffers
    // and renderpasses to the resource lookups
    //
    let executor = RenderGraphExecutor::new(
        &device_context,
        &resource_context,
        graph,
        swapchain_surface_info,
        graph_callbacks,
    )?;

    let ui_renderpass = executor.renderpass_resource(ui_pass.node);
    Ok(BuildRenderGraphResult {
        executor,
        ui_renderpass,
    })
}
