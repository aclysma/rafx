use ash::vk;
use crate::vk_description as dsc;

mod graph;
use graph::*;
pub use graph::RenderGraph;

mod graph_image;
use graph_image::*;
pub use graph_image::RenderGraphImageUsageId;
pub use graph_image::RenderGraphImageConstraint;
pub use graph_image::RenderGraphImageSpecification;

mod graph_node;
use graph_node::*;

mod prepared_graph;
pub use prepared_graph::RenderGraphPlan;
pub use prepared_graph::RenderGraphNodeCallbacks;
pub use prepared_graph::PreparedRenderGraph;
pub use prepared_graph::RenderGraphExecutor;

use crate::vk_description::SwapchainSurfaceInfo;
use renderer_shell_vulkan::MsaaLevel;

#[test]
fn graph_smoketest() {
    // - Should there be some way to "pull forward" future constraints to some point?
    // - Maybe we just rely on programmer setting the constraint where they want it since they
    //   can check what the swapchain image or whatever would be anyways. Likely a requirement
    //   since they'd need to set up the shaders properly for it.
    // - Don't need to merge renderpasses yet
    // - Could make renderpass merging manual/opt-in and assert if it can't merge
    // - Or just do it automatically

    let color_format = vk::Format::R8G8B8A8_SRGB;
    let depth_format = vk::Format::D32_SFLOAT;
    let swapchain_format = vk::Format::R8G8B8A8_SRGB;
    let msaa_level = MsaaLevel::Sample4;
    let samples = msaa_level.into();
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
            },
            dsc::ImageLayout::PresentSrcKhr,
            vk::AccessFlags::empty(),
            vk::PipelineStageFlags::empty(),
            vk::ImageAspectFlags::COLOR,
        );

    //println!("{:#?}", graph);
    let swapchain_surface_info = SwapchainSurfaceInfo {
        color_format,
        depth_format,
        msaa_level: msaa_level,
        extents: vk::Extent2D {
            width: 900,
            height: 600,
        },
        surface_format: vk::SurfaceFormatKHR {
            format: swapchain_format,
            color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
        },
    };
    graph.into_plan(&swapchain_surface_info);
}
