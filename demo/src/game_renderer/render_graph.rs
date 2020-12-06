use crate::features::mesh::ShadowMapRenderView;
use crate::phases::{OpaqueRenderPhase, ShadowMapRenderPhase, UiRenderPhase};
use crate::render_contexts::RenderJobWriteContext;
use crate::VkDeviceContext;
use ash::prelude::VkResult;
use ash::version::DeviceV1_0;
use ash::vk;
use rafx::graph::*;
use rafx::nodes::{PreparedRenderData, RenderView};
use rafx::resources::ResourceContext;
use rafx::resources::{vk_description as dsc, VertexDataSetLayout};
use rafx::resources::{ImageViewResource, MaterialPassResource, ResourceArc};
use rafx::vulkan::SwapchainInfo;

lazy_static::lazy_static! {
    pub static ref EMPTY_VERTEX_LAYOUT : VertexDataSetLayout = {
        VertexDataSetLayout::new(vec![])
    };
}

pub struct BuildRenderGraphResult {
    pub shadow_map_image_views: Vec<ResourceArc<ImageViewResource>>,
    pub executor: RenderGraphExecutor<RenderGraphUserContext>,
}

// Any data you want available within rendergraph execution callbacks should go here. This can
// include data that is not known until later after the extract/prepare phases have completed.
pub struct RenderGraphUserContext {
    pub prepared_render_data: Box<PreparedRenderData<RenderJobWriteContext>>,
}

struct ShadowMapPass {
    node: RenderGraphNodeId,
    depth: RenderGraphImageUsageId,
}

fn shadow_map_pass(
    graph: &mut RenderGraphBuilder,
    graph_callbacks: &mut RenderGraphNodeCallbacks<RenderGraphUserContext>,
    render_view: &RenderView,
    depth_image: RenderGraphImageUsageId,
    layer: usize,
) -> ShadowMapPass {
    let node = graph.add_node("Shadow", RenderGraphQueue::DefaultGraphics);

    let depth = graph.modify_depth_attachment(
        node,
        depth_image,
        Some(vk::ClearDepthStencilValue {
            depth: 0.0,
            stencil: 0,
        }),
        RenderGraphImageConstraint::default(),
        RenderGraphImageSubresourceRange::NoMipsSingleLayer(layer as u32),
    );
    graph.set_image_name(depth, "depth");

    graph_callbacks.add_renderphase_dependency::<ShadowMapRenderPhase>(node);

    let render_view = render_view.clone();
    graph_callbacks.set_renderpass_callback(node, move |args, user_context| {
        let mut write_context = RenderJobWriteContext::from_graph_visit_render_pass_args(&args);
        user_context
            .prepared_render_data
            .write_view_phase::<ShadowMapRenderPhase>(&render_view, &mut write_context);
        Ok(())
    });

    ShadowMapPass { node, depth }
}

enum ShadowMapImageResources {
    Single(RenderGraphImageUsageId),
    Cube(RenderGraphImageUsageId),
}

pub fn build_render_graph(
    device_context: &VkDeviceContext,
    resource_context: &ResourceContext,
    swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    swapchain_info: &SwapchainInfo,
    swapchain_image: ResourceArc<ImageViewResource>,
    main_view: RenderView,
    shadow_map_views: &[ShadowMapRenderView],
    bloom_extract_material_pass: ResourceArc<MaterialPassResource>,
    bloom_blur_material_pass: ResourceArc<MaterialPassResource>,
    bloom_combine_material_pass: ResourceArc<MaterialPassResource>,
) -> VkResult<BuildRenderGraphResult> {
    profiling::scope!("Build Render Graph");

    let enable_hdr = true;

    //TODO: Fix this back to be color format - need to happen in the combine pass
    let color_format = if enable_hdr {
        swapchain_surface_info.color_format
    } else {
        swapchain_surface_info.surface_format.format
    };

    let depth_format = swapchain_surface_info.depth_format;
    let swapchain_format = swapchain_surface_info.surface_format.format;
    let samples = swapchain_surface_info.msaa_level.into();

    let mut graph = RenderGraphBuilder::default();
    let mut graph_callbacks = RenderGraphNodeCallbacks::<RenderGraphUserContext>::default();

    let mut shadow_map_passes = Vec::default();
    for shadow_map_view in shadow_map_views {
        match shadow_map_view {
            ShadowMapRenderView::Single(render_view) => {
                let shadow_map_node =
                    graph.add_node("create shadowmap", RenderGraphQueue::DefaultGraphics);
                let depth_image = graph.add_image(
                    shadow_map_node,
                    RenderGraphImageConstraint {
                        format: Some(depth_format),
                        extents: Some(RenderGraphImageExtents::Custom(
                            render_view.extents_width(),
                            render_view.extents_height(),
                            1,
                        )),
                        ..Default::default()
                    },
                    dsc::ImageViewType::Type2D,
                );

                let shadow_map_pass = shadow_map_pass(
                    &mut graph,
                    &mut graph_callbacks,
                    render_view,
                    depth_image,
                    0,
                );
                shadow_map_passes.push(ShadowMapImageResources::Single(shadow_map_pass.depth));
            }
            ShadowMapRenderView::Cube(render_view) => {
                let cube_map_node =
                    graph.add_node("create cube shadowmap", RenderGraphQueue::DefaultGraphics);
                let mut cube_map_image = graph.add_image(
                    cube_map_node,
                    RenderGraphImageConstraint {
                        format: Some(depth_format),
                        create_flags: vk::ImageCreateFlags::CUBE_COMPATIBLE,
                        layer_count: Some(6),
                        extents: Some(RenderGraphImageExtents::Custom(
                            render_view[0].extents_width(),
                            render_view[0].extents_height(),
                            1,
                        )),
                        ..Default::default()
                    },
                    dsc::ImageViewType::Cube,
                );

                for i in 0..6 {
                    cube_map_image = shadow_map_pass(
                        &mut graph,
                        &mut graph_callbacks,
                        &render_view[i],
                        cube_map_image,
                        i,
                    )
                    .depth;
                }

                //
                shadow_map_passes.push(ShadowMapImageResources::Cube(cube_map_image));
            }
        }
    }

    let opaque_pass = {
        struct OpaquePass {
            node: RenderGraphNodeId,
            color: RenderGraphImageUsageId,
            depth: RenderGraphImageUsageId,
            shadow_maps: Vec<RenderGraphImageUsageId>,
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
                depth: 0.0,
                stencil: 0,
            }),
            RenderGraphImageConstraint {
                samples: Some(samples),
                format: Some(depth_format),
                ..Default::default()
            },
        );
        graph.set_image_name(depth, "depth");

        let mut shadow_maps = Vec::with_capacity(shadow_map_passes.len());
        for shadow_map_pass in &shadow_map_passes {
            let sampled_image = match shadow_map_pass {
                ShadowMapImageResources::Single(image) => graph.sample_image(
                    node,
                    *image,
                    Default::default(),
                    RenderGraphImageSubresourceRange::AllMipsAllLayers,
                    dsc::ImageViewType::Type2D,
                ),
                ShadowMapImageResources::Cube(cube_map_image) => graph.sample_image(
                    node,
                    *cube_map_image,
                    Default::default(),
                    RenderGraphImageSubresourceRange::AllMipsAllLayers,
                    dsc::ImageViewType::Cube,
                ),
            };
            shadow_maps.push(sampled_image);
        }

        graph_callbacks.add_renderphase_dependency::<OpaqueRenderPhase>(node);

        let main_view = main_view.clone();
        graph_callbacks.set_renderpass_callback(node, move |args, user_context| {
            let mut write_context = RenderJobWriteContext::from_graph_visit_render_pass_args(&args);
            user_context
                .prepared_render_data
                .write_view_phase::<OpaqueRenderPhase>(&main_view, &mut write_context);
            Ok(())
        });

        OpaquePass {
            node,
            color,
            depth,
            shadow_maps,
        }
    };

    let previous_pass_color = if enable_hdr {
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
                Default::default(),
                Default::default(),
            );

            graph_callbacks.set_renderpass_callback(node, move |args, _user_context| {
                // Get the color image from before
                let sample_image = args.graph_context.image_view(sample_image);

                // Get the pipeline
                let pipeline = args
                    .graph_context
                    .resource_context()
                    .graphics_pipeline_cache()
                    .get_or_create_graphics_pipeline(
                        &bloom_extract_material_pass,
                        args.renderpass_resource,
                        &args
                            .framebuffer_resource
                            .get_raw()
                            .framebuffer_key
                            .framebuffer_meta,
                        &EMPTY_VERTEX_LAYOUT,
                    )?;

                // Set up a descriptor set pointing at the image so we can sample from it
                let mut descriptor_set_allocator = args
                    .graph_context
                    .resource_context()
                    .create_descriptor_set_allocator();

                let descriptor_set_layouts =
                    &pipeline.get_raw().pipeline_layout.get_raw().descriptor_sets;
                let bloom_extract_material_dyn_set = descriptor_set_allocator
                    .create_descriptor_set(
                        &descriptor_set_layouts
                            [shaders::bloom_extract_frag::TEX_DESCRIPTOR_SET_INDEX],
                        shaders::bloom_extract_frag::DescriptorSet0Args {
                            tex: sample_image.as_ref().unwrap(),
                        },
                    )?;

                // Explicit flush since we're going to use the descriptors immediately
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
                        shaders::bloom_extract_frag::TEX_DESCRIPTOR_SET_INDEX as u32,
                        &[bloom_extract_material_dyn_set.get()],
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

                let sample_image = graph.sample_image(
                    node,
                    blur_src,
                    Default::default(),
                    Default::default(),
                    Default::default(),
                );
                graph.set_image_name(blur_src, "blur_src");

                let bloom_blur_material_pass = bloom_blur_material_pass.clone();
                graph_callbacks.set_renderpass_callback(node, move |args, _user_context| {
                    // Get the color image from before
                    let sample_image = args.graph_context.image_view(sample_image);

                    // Get the pipeline
                    let pipeline = args
                        .graph_context
                        .resource_context()
                        .graphics_pipeline_cache()
                        .get_or_create_graphics_pipeline(
                            &bloom_blur_material_pass,
                            args.renderpass_resource,
                            &args
                                .framebuffer_resource
                                .get_raw()
                                .framebuffer_key
                                .framebuffer_meta,
                            &EMPTY_VERTEX_LAYOUT,
                        )?;

                    let descriptor_set_layouts =
                        &pipeline.get_raw().pipeline_layout.get_raw().descriptor_sets;

                    // Set up a descriptor set pointing at the image so we can sample from it
                    let mut descriptor_set_allocator = args
                        .graph_context
                        .resource_context()
                        .create_descriptor_set_allocator();

                    let bloom_blur_material_dyn_set = descriptor_set_allocator
                        .create_descriptor_set(
                            &descriptor_set_layouts
                                [shaders::bloom_blur_frag::TEX_DESCRIPTOR_SET_INDEX],
                            shaders::bloom_blur_frag::DescriptorSet0Args {
                                tex: sample_image.as_ref().unwrap(),
                                config: &shaders::bloom_blur_frag::ConfigUniform {
                                    horizontal: blur_pass_index % 2,
                                    ..Default::default()
                                },
                            },
                        )?;

                    // Explicit flush since we're going to use the descriptors immediately
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
                            shaders::bloom_blur_frag::CONFIG_DESCRIPTOR_SET_INDEX as u32,
                            &[bloom_blur_material_dyn_set.get()],
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

            let sdr_image = graph.sample_image(
                node,
                bloom_extract_pass.sdr_image,
                Default::default(),
                Default::default(),
                Default::default(),
            );
            graph.set_image_name(sdr_image, "sdr");

            let hdr_image = graph.sample_image(
                node,
                bloom_blur_pass.color,
                Default::default(),
                Default::default(),
                Default::default(),
            );
            graph.set_image_name(hdr_image, "hdr");

            graph_callbacks.set_renderpass_callback(node, move |args, _user_context| {
                // Get the color image from before
                let sdr_image = args.graph_context.image_view(sdr_image).unwrap();
                let hdr_image = args.graph_context.image_view(hdr_image).unwrap();

                // Get the pipeline
                let pipeline = args
                    .graph_context
                    .resource_context()
                    .graphics_pipeline_cache()
                    .get_or_create_graphics_pipeline(
                        &bloom_combine_material_pass,
                        args.renderpass_resource,
                        &args
                            .framebuffer_resource
                            .get_raw()
                            .framebuffer_key
                            .framebuffer_meta,
                        &EMPTY_VERTEX_LAYOUT,
                    )?;

                // Set up a descriptor set pointing at the image so we can sample from it
                let mut descriptor_set_allocator = args
                    .graph_context
                    .resource_context()
                    .create_descriptor_set_allocator();

                let descriptor_set_layouts =
                    &pipeline.get_raw().pipeline_layout.get_raw().descriptor_sets;
                let bloom_combine_material_dyn_set = descriptor_set_allocator
                    .create_descriptor_set(
                        &descriptor_set_layouts
                            [shaders::bloom_combine_frag::IN_COLOR_DESCRIPTOR_SET_INDEX],
                        shaders::bloom_combine_frag::DescriptorSet0Args {
                            in_color: &sdr_image,
                            in_blur: &hdr_image,
                        },
                    )?;

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
                        shaders::bloom_combine_frag::IN_COLOR_DESCRIPTOR_SET_INDEX as u32,
                        &[bloom_combine_material_dyn_set.get()],
                        &[],
                    );

                    device.cmd_draw(command_buffer, 3, 1, 0, 0);
                }

                Ok(())
            });

            BloomCombinePass { node, color }
        };

        bloom_combine_pass.color
    } else {
        opaque_pass.color
    };

    let ui_pass = {
        struct UiPass {
            node: RenderGraphNodeId,
            color: RenderGraphImageUsageId,
        }

        // This node has a single color attachment
        let node = graph.add_node("Ui", RenderGraphQueue::DefaultGraphics);
        let color = graph.modify_color_attachment(
            node,
            previous_pass_color,
            0,
            None,
            Default::default(),
            Default::default(),
        );
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
            create_flags: Default::default(),
            extents: RenderGraphImageExtents::MatchSurface,
            layer_count: 1,
            mip_count: 1,
        },
        Default::default(),
        Default::default(),
        dsc::ImageLayout::PresentSrcKhr,
        vk::AccessFlags::empty(),
        vk::PipelineStageFlags::empty(),
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

    let shadow_map_image_views = opaque_pass
        .shadow_maps
        .iter()
        .map(|&x| executor.image_view_resource(x).unwrap())
        .collect();

    Ok(BuildRenderGraphResult {
        shadow_map_image_views,
        executor,
    })
}
