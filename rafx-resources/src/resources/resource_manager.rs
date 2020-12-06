use super::dyn_resource_allocator;
use super::pipeline_cache;
use super::resource_lookup;
use crate::{
    DescriptorSetAllocatorProvider, DescriptorSetAllocatorRef, DynResourceAllocatorSet,
    GraphicsPipelineCache,
};
use ash::prelude::*;
use rafx_shell_vulkan::VkDeviceContext;

use crate::graph::RenderGraphCache;
use crate::resources::command_buffers::DynCommandWriterAllocator;
use crate::resources::descriptor_sets::DescriptorSetAllocatorManager;
use crate::resources::dyn_resource_allocator::{
    DynResourceAllocatorSetManager, DynResourceAllocatorSetProvider,
};
use crate::resources::resource_lookup::ResourceLookupSet;
use rafx_nodes::RenderRegistry;
use std::sync::Arc;

//TODO: Support descriptors that can be different per-view
//TODO: Support dynamic descriptors tied to command buffers?
//TODO: Support data inheritance for descriptors

#[derive(Debug)]
pub struct ResourceManagerMetrics {
    pub dyn_resource_metrics: dyn_resource_allocator::ResourceMetrics,
    pub resource_metrics: resource_lookup::ResourceMetrics,
    pub graphics_pipeline_cache_metrics: pipeline_cache::GraphicsPipelineCacheMetrics,
}

struct ResourceContextInner {
    descriptor_set_allocator_provider: DescriptorSetAllocatorProvider,
    dyn_resources_allocator_provider: DynResourceAllocatorSetProvider,
    dyn_commands_allocator: DynCommandWriterAllocator,
    resources: ResourceLookupSet,
    graphics_pipeline_cache: GraphicsPipelineCache,
    render_graph_cache: RenderGraphCache,
}

#[derive(Clone)]
pub struct ResourceContext {
    inner: Arc<ResourceContextInner>,
}

impl ResourceContext {
    pub fn resources(&self) -> &ResourceLookupSet {
        &self.inner.resources
    }

    pub fn graphics_pipeline_cache(&self) -> &GraphicsPipelineCache {
        &self.inner.graphics_pipeline_cache
    }

    pub fn render_graph_cache(&self) -> &RenderGraphCache {
        &self.inner.render_graph_cache
    }

    pub fn dyn_command_writer_allocator(&self) -> DynCommandWriterAllocator {
        self.inner.dyn_commands_allocator.clone()
    }

    pub fn create_dyn_resource_allocator_set(&self) -> DynResourceAllocatorSet {
        self.inner.dyn_resources_allocator_provider.get_allocator()
    }

    pub fn create_descriptor_set_allocator(&self) -> DescriptorSetAllocatorRef {
        self.inner.descriptor_set_allocator_provider.get_allocator()
    }
}

pub struct ResourceManager {
    render_registry: RenderRegistry,
    dyn_resources: DynResourceAllocatorSetManager,
    dyn_commands: DynCommandWriterAllocator,
    resources: ResourceLookupSet,
    render_graph_cache: RenderGraphCache,
    descriptor_set_allocator: DescriptorSetAllocatorManager,
    graphics_pipeline_cache: GraphicsPipelineCache,
}

impl ResourceManager {
    pub fn new(
        device_context: &VkDeviceContext,
        render_registry: &RenderRegistry,
    ) -> Self {
        let resources = ResourceLookupSet::new(
            device_context,
            rafx_shell_vulkan::MAX_FRAMES_IN_FLIGHT as u32,
        );

        ResourceManager {
            render_registry: render_registry.clone(),
            dyn_commands: DynCommandWriterAllocator::new(
                device_context,
                rafx_shell_vulkan::MAX_FRAMES_IN_FLIGHT as u32,
            ),
            dyn_resources: DynResourceAllocatorSetManager::new(
                device_context,
                rafx_shell_vulkan::MAX_FRAMES_IN_FLIGHT as u32,
            ),
            resources: resources.clone(),
            render_graph_cache: RenderGraphCache::new(
                rafx_shell_vulkan::MAX_FRAMES_IN_FLIGHT as u32,
            ),
            descriptor_set_allocator: DescriptorSetAllocatorManager::new(device_context),
            graphics_pipeline_cache: GraphicsPipelineCache::new(render_registry, resources),
        }
    }

    pub fn resource_context(&self) -> ResourceContext {
        let inner = ResourceContextInner {
            descriptor_set_allocator_provider: self
                .descriptor_set_allocator
                .create_allocator_provider(),
            dyn_resources_allocator_provider: self.dyn_resources.create_allocator_provider(),
            dyn_commands_allocator: self.dyn_commands.clone(),
            resources: self.resources.clone(),
            graphics_pipeline_cache: self.graphics_pipeline_cache.clone(),
            render_graph_cache: self.render_graph_cache.clone(),
        };

        ResourceContext {
            inner: Arc::new(inner),
        }
    }

    pub fn resources(&self) -> &ResourceLookupSet {
        &self.resources
    }

    pub fn graphics_pipeline_cache(&self) -> &GraphicsPipelineCache {
        &self.graphics_pipeline_cache
    }

    pub fn dyn_command_writer_allocator(&self) -> &DynCommandWriterAllocator {
        &self.dyn_commands
    }

    pub fn create_dyn_resource_allocator_set(&self) -> DynResourceAllocatorSet {
        self.dyn_resources.get_allocator()
    }

    pub fn create_dyn_resource_allocator_provider(&self) -> DynResourceAllocatorSetProvider {
        self.dyn_resources.create_allocator_provider()
    }

    pub fn create_descriptor_set_allocator(&self) -> DescriptorSetAllocatorRef {
        self.descriptor_set_allocator.get_allocator()
    }

    pub fn create_descriptor_set_allocator_provider(&self) -> DescriptorSetAllocatorProvider {
        self.descriptor_set_allocator.create_allocator_provider()
    }

    pub fn render_registry(&self) -> &RenderRegistry {
        &self.render_registry
    }

    pub fn metrics(&self) -> ResourceManagerMetrics {
        let dyn_resource_metrics = self.dyn_resources.metrics();
        let resource_metrics = self.resources.metrics();
        let graphics_pipeline_cache_metrics = self.graphics_pipeline_cache.metrics();

        ResourceManagerMetrics {
            dyn_resource_metrics,
            resource_metrics,
            graphics_pipeline_cache_metrics,
        }
    }

    #[profiling::function]
    pub fn on_frame_complete(&mut self) -> VkResult<()> {
        self.render_graph_cache.on_frame_complete();
        self.graphics_pipeline_cache.on_frame_complete();
        self.resources.on_frame_complete()?;
        self.dyn_commands.on_frame_complete()?;
        self.dyn_resources.on_frame_complete()?;
        self.descriptor_set_allocator.on_frame_complete();
        Ok(())
    }
}

impl Drop for ResourceManager {
    fn drop(&mut self) {
        log::info!("Cleaning up resource manager");
        log::trace!("Resource Manager Metrics:\n{:#?}", self.metrics());

        // Wipe caches to ensure we don't keep anything alive
        self.render_graph_cache.clear();
        self.graphics_pipeline_cache.clear_all_pipelines();

        // Drop all descriptors. These bind to raw resources, so we need to drop them before
        // dropping resources
        self.descriptor_set_allocator.destroy().unwrap();

        // Now drop all resources with a zero ref count and warn for any resources that remain
        self.resources.destroy().unwrap();
        self.dyn_resources.destroy().unwrap();

        log::info!("Dropping resource manager");
        log::trace!("Resource Manager Metrics:\n{:#?}", self.metrics());
    }
}
