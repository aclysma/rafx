use ash::vk;
use renderer::assets::vk_description as dsc;
use renderer::assets::graph::*;
use renderer::assets::resources::ResourceLookupSet;
use crate::VkDeviceContext;
use ash::prelude::VkResult;

pub fn setup_graph(
    swapchain_info: &dsc::SwapchainSurfaceInfo,
    device_context: &VkDeviceContext,
    resources: &mut ResourceLookupSet,
) -> VkResult<()> {
    let color_format = swapchain_info.color_format;
    let depth_format = swapchain_info.depth_format;
    let swapchain_format = swapchain_info.surface_format.format;
    //let samples = swapchain_info.msaa_level.into();
    let samples = vk::SampleCountFlags::TYPE_1;
    let queue = 0;

    let mut graph = RenderGraph::default();
    let swapchain_image = ();

    let opaque_pass = {
        struct Opaque {
            color: RenderGraphImageUsageId,
            depth: RenderGraphImageUsageId,
        }

        let mut node = graph.add_node();
        node.set_name("Opaque");
        let color = node.create_color_attachment(
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

        let depth = node.create_depth_attachment(
            Some(vk::ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            }),
            RenderGraphImageConstraint {
                samples: Some(samples),
                format: Some(depth_format),
                queue: Some(queue),
                ..Default::default()
            },
        );

        graph.configure_image(color).set_name("color");
        graph.configure_image(depth).set_name("depth");

        Opaque { color, depth }
    };

    let transparent_pass = {
        struct Transparent {
            color: RenderGraphImageUsageId,
        }

        let mut node = graph.add_node();
        node.set_name("Transparent");

        let color = node.modify_color_attachment(
            opaque_pass.color,
            0,
            RenderGraphImageConstraint {
                //layout: Some(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL),
                // samples: Some(vk::SampleCountFlags::TYPE_1),
                // format: Some(swapchain_format),
                // queue: Some(queue),
                ..Default::default()
            },
        );

        node.read_depth_attachment(opaque_pass.depth, Default::default());

        Transparent { color }
    };

    let swapchain_output_image_id = graph
        .configure_image(transparent_pass.color)
        .set_output_image(
            swapchain_image,
            RenderGraphImageSpecification {
                samples: vk::SampleCountFlags::TYPE_1,
                format: swapchain_format,
                queue,
                aspect_flags: vk::ImageAspectFlags::COLOR,
                usage_flags: vk::ImageUsageFlags::empty(),
            },
            dsc::ImageLayout::PresentSrcKhr,
            vk::AccessFlags::empty(),
            vk::PipelineStageFlags::empty(),
            vk::ImageAspectFlags::COLOR,
        );

    //println!("{:#?}", graph);
    let prepared_render_graph = graph.prepare(swapchain_info);

    // for (physical_image_id, spec) in prepared_render_graph.intermediate_images {
    //     VkImage
    // }

    let mut framebuffer_allocator = FramebufferAllocator::new(device_context.clone());
    framebuffer_allocator.allocate_resources(&prepared_render_graph, resources, swapchain_info)?;
    Ok(())
}
