use crate::graph::{
    RenderGraphBuilder, RenderGraphImageConstraint, RenderGraphImageExtents,
    RenderGraphImageSpecification, RenderGraphImageUsageId, RenderGraphQueue,
};
use crate::vk_description::SwapchainSurfaceInfo;
use crate::{
    vk_description as dsc, ImageResource, ImageViewResource, ResourceArc, ResourceId,
    ResourceWithHash,
};
use ash::vk;
use crossbeam_channel::{Receiver, Sender};
use rafx_shell_vulkan::{MsaaLevel, VkImageRaw, VkResource};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TestResourceIndex(u64);

impl From<ResourceId> for TestResourceIndex {
    fn from(resource_id: ResourceId) -> Self {
        TestResourceIndex(resource_id.0)
    }
}

impl Into<ResourceId> for TestResourceIndex {
    fn into(self) -> ResourceId {
        ResourceId(self.0)
    }
}

#[derive(Default)]
pub struct TestResourceAllocatorInner<ResourceT>
where
    ResourceT: VkResource + Clone,
{
    next_index: AtomicU64,
    active_count: Arc<AtomicU32>,
    phantom_data: PhantomData<ResourceT>,
}

#[derive(Clone)]
pub struct TestResourceAllocator<ResourceT>
where
    ResourceT: VkResource + Clone,
{
    inner: Arc<TestResourceAllocatorInner<ResourceT>>,
    drop_tx: Sender<ResourceWithHash<ResourceT>>,
    drop_rx: Receiver<ResourceWithHash<ResourceT>>,
}

impl<ResourceT> Drop for TestResourceAllocator<ResourceT>
where
    ResourceT: VkResource + Clone,
{
    fn drop(&mut self) {
        for _ in self.drop_rx.try_iter() {}
    }
}

impl<ResourceT> TestResourceAllocator<ResourceT>
where
    ResourceT: VkResource + Clone + std::fmt::Debug,
{
    fn new() -> Self {
        let inner = TestResourceAllocatorInner {
            next_index: AtomicU64::new(1),
            active_count: Arc::new(AtomicU32::new(0)),
            phantom_data: Default::default(),
        };

        let (drop_tx, drop_rx) = crossbeam_channel::unbounded();

        TestResourceAllocator {
            inner: Arc::new(inner),
            drop_tx,
            drop_rx,
        }
    }

    fn insert(
        &self,
        resource: ResourceT,
    ) -> ResourceArc<ResourceT> {
        // This index is not strictly necessary. However, we do want to be compatible with ResourceArc,
        // and in other usecases a working index is necessary. Since we have the index anyways, we
        // might as well produce some sort of index if only to make logging easier to follow
        let resource_index =
            TestResourceIndex(self.inner.next_index.fetch_add(1, Ordering::Relaxed));
        self.inner.active_count.fetch_add(1, Ordering::Relaxed);

        log::trace!(
            "insert resource {} {:?}",
            core::any::type_name::<ResourceT>(),
            resource
        );

        ResourceArc::new(resource, resource_index.into(), self.drop_tx.clone())
    }

    fn handle_dropped_resources(&mut self) {
        for _ in self.drop_rx.try_iter() {}
    }
}

impl<ResourceT> Default for TestResourceAllocator<ResourceT>
where
    ResourceT: VkResource + Clone + std::fmt::Debug,
{
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Default)]
struct TestImageViewAllocator {
    image_allocator: TestResourceAllocator<ImageResource>,
    image_view_allocator: TestResourceAllocator<ImageViewResource>,
}

impl TestImageViewAllocator {
    fn create_image(&self) -> ResourceArc<ImageViewResource> {
        let image = self.image_allocator.insert(ImageResource {
            image: VkImageRaw {
                image: vk::Image::null(),
                allocation: None,
            },
            image_key: None,
        });

        self.image_view_allocator.insert(ImageViewResource {
            image,
            image_view: vk::ImageView::null(),
            image_view_key: None,
            image_view_meta: dsc::ImageViewMeta::default_2d_no_mips_or_layers(
                dsc::Format::UNDEFINED,
                dsc::ImageAspectFlags::empty(),
            ),
        })
    }

    fn handle_dropped_resources(&mut self) {
        self.image_view_allocator.handle_dropped_resources();
        self.image_allocator.handle_dropped_resources();
    }
}

impl Drop for TestImageViewAllocator {
    fn drop(&mut self) {
        self.handle_dropped_resources();
    }
}

#[test]
fn graph_smoketest() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Trace)
        .try_init();

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

    let mut graph = RenderGraphBuilder::default();

    let test_image_resources = TestImageViewAllocator::default();
    {
        let swapchain_image = test_image_resources.create_image();

        let opaque_pass = {
            struct Opaque {
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

            Opaque { color, depth }
        };

        let transparent_pass = {
            struct Transparent {
                color: RenderGraphImageUsageId,
            }

            let node = graph.add_node("Transparent", RenderGraphQueue::DefaultGraphics);

            let color = graph.modify_color_attachment(
                node,
                opaque_pass.color,
                0,
                None,
                Default::default(),
                Default::default(),
            );
            graph.set_image_name(color, "color");

            graph.read_depth_attachment(
                node,
                opaque_pass.depth,
                Default::default(),
                Default::default(),
            );

            Transparent { color }
        };

        graph.set_output_image(
            transparent_pass.color,
            swapchain_image,
            RenderGraphImageSpecification {
                samples: vk::SampleCountFlags::TYPE_1,
                format: swapchain_format,
                aspect_flags: vk::ImageAspectFlags::COLOR,
                usage_flags: vk::ImageUsageFlags::COLOR_ATTACHMENT,
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

        //println!("{:#?}", graph);
        let swapchain_surface_info = SwapchainSurfaceInfo {
            color_format,
            depth_format,
            msaa_level,
            extents: vk::Extent2D {
                width: 900,
                height: 600,
            },
            surface_format: vk::SurfaceFormatKHR {
                format: swapchain_format,
                color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
            },
        };
        let x = graph.build_plan(&swapchain_surface_info);
        std::mem::drop(x);
    }

    std::mem::drop(test_image_resources);
    println!("done");
}
