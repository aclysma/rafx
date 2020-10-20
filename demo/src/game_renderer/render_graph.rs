use ash::vk;
use renderer::assets::vk_description as dsc;
use renderer::assets::graph::*;
use renderer::assets::resources::ResourceManager;
use crate::VkDeviceContext;
use ash::prelude::VkResult;
use renderer::assets::resources::{ResourceArc, ImageViewResource, DynCommandWriter};
use crate::render_contexts::{RenderJobWriteContextFactory, RenderJobWriteContext};
use renderer::nodes::{PreparedRenderData, RenderView};
use crate::phases::OpaqueRenderPhase;
use renderer::vulkan::SwapchainInfo;

/*
mod x {
    use std::collections::HashMap;

    // This is a map of callbacks that get triggered later by passing a &T. That &T
    // is provided to the callbacks
    type Callback<T> = dyn Fn(&mut T) -> u32 + Send;
    struct Callbacks<T> {
        callbacks: HashMap<u32, Box<Callback<T>>>
    }

    // Have to impl this manually because <T> doesn't impl Default...
    impl<T> Default for Callbacks<T> {
        fn default() -> Self {
            Callbacks {
                callbacks: Default::default()
            }
        }
    }

    fn test() {
        // This struct combines the parameters that I don't know now, but that I will know later and
        // want to pass into the callback
        struct ContextObject<'a> {
            data1: &'a mut Vec<u32>,
            data2: &'a mut Vec<u8>,
            // Imagine I have a handful of things I need to pass in here that I can't simply copy/clone
        }

        // NOTE: It's problematic that I need <'a> on this struct because now any other
        // struct that needs to pass a Callbacks<ContextObject<'a>> around has to have
        // a lifetime <'a> too
        //struct SomethingContainingCallbacks {
        //    callbacks: Callbacks<ContextObject<'a>>
        //}

        // Set up a callback that will use that data later
        let mut callbacks = Callbacks::<ContextObject>::default();
        callbacks.callbacks.insert(0, Box::new(|context_obj| {
            println!("data is {:?}", context_obj.data1);
            context_obj.data2.push(0);
            0
        }));

        // Now I have some data and want to hit callbacks with it
        let mut data1 = Vec::default();
        let mut data2 = Vec::default();

        // So create a context object that has references to them
        let mut context = ContextObject {
            data1: &mut data1,
            data2: &mut data2
        };

        // Trigger the callback
        (callbacks.callbacks[&0])(&mut context);

        std::mem::drop(context);
        std::mem::drop(callbacks);

        // ERROR:
        // * `data1` dropped here while still borrowed
        //   borrow might be used here, when `callbacks` is dropped and runs the destructor for type `Callbacks<test::ContextObject<'_>>`
        //
        // So presumably rust thinks a callback within callbacks holds a borrow to data1?
    }
}
*/













// Any data you want available within rendergraph execution callbacks should go here. This can
// include data that is not known until later after the extract/prepare phases have completed.
pub struct RenderGraphExecuteContext {
    pub prepared_render_data: Box<PreparedRenderData<RenderJobWriteContext>>,
    pub view: RenderView,
    pub write_context_factory: RenderJobWriteContextFactory,
    pub command_writer: DynCommandWriter,
}

pub fn build_render_graph(
    swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    device_context: &VkDeviceContext,
    resource_manager: &mut ResourceManager,
    swapchain_info: &SwapchainInfo,
    swapchain_image: ResourceArc<ImageViewResource>,
) -> VkResult<RenderGraphExecutor<RenderGraphExecuteContext>> {
    //let color_format = swapchain_surface_info.color_format;
    let color_format = swapchain_surface_info.surface_format.format;
    let depth_format = swapchain_surface_info.depth_format;
    let swapchain_format = swapchain_surface_info.surface_format.format;
    //let samples = swapchain_surface_info.msaa_level.into();
    let samples = vk::SampleCountFlags::TYPE_1;
    let queue = 0;

    let mut graph = RenderGraph::default();
    let mut graph_callbacks = RenderGraphNodeCallbacks::<RenderGraphExecuteContext>::default();

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

        // node.set_pass_callback(|prepared_render_data, view, write_context_factory, command_writer| {
        //     let mut write_context = write_context_factory.create_context(command_buffer);
        //     prepared_render_data.write_view_phase::<OpaqueRenderPhase>(&view, &mut write_context);
        //     Ok(())
        // });

        graph_callbacks.add_renderpass_callback(node.id(), |command_buffer, context| {
            let mut write_context = context.write_context_factory.create_context(command_buffer);
            context.prepared_render_data.write_view_phase::<OpaqueRenderPhase>(&context.view, &mut write_context);
            Ok(())
        });

        graph.configure_image(color).set_name("color");
        graph.configure_image(depth).set_name("depth");

        Opaque { color, depth }
    };
/*
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
                ..Default::default()
            },
        );

        node.read_depth_attachment(opaque_pass.depth, Default::default());


        graph_callbacks.add_renderpass_callback(node.id(), |command_buffer, context| {
            let mut write_context = context.write_context_factory.create_context(command_buffer);
            context.prepared_render_data.write_view_phase::<OpaqueRenderPhase>(&context.view, &mut write_context);
            Ok(())
        });


        // node.set_pass_callback(|prepared_render_data, view, write_context_factory, command_writer| {
        //     let mut write_context = write_context_factory.create_context(command_buffer);
        //     prepared_render_data.write_view_phase::<TransparentRenderPhase>(&view, &mut write_context);
        //     Ok(())
        // });

        Transparent { color }
    };
*/
    let swapchain_output_image_id = graph
        .configure_image(opaque_pass.color)
        .set_output_image(
            swapchain_image,
            RenderGraphImageSpecification {
                samples: vk::SampleCountFlags::TYPE_1,
                format: swapchain_format,
                queue,
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
    let mut executor = RenderGraphExecutor::new(
        &device_context,
        graph,
        resource_manager.resources_mut(),
        swapchain_surface_info,
        graph_callbacks
    )?;

    // //
    // // Execute the graph. The context can include arbitrary data
    // //
    // let write_context = RenderGraphExecuteContext {
    //
    // };
    // executor.execute_graph(
    //     &resource_manager.create_dyn_command_writer_allocator(),
    //     &write_context
    // )?;


    Ok(executor)
}
