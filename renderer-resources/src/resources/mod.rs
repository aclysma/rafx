mod resource_arc;
pub use resource_arc::ResourceArc;
#[cfg(test)]
pub(crate) use resource_arc::ResourceWithHash;
pub(crate) use resource_arc::ResourceId;

mod resource_lookup;
pub use resource_lookup::ResourceLookupSet;
pub use resource_lookup::ResourceHash;
pub use resource_lookup::DescriptorSetLayoutResource;
pub use resource_lookup::ImageKey;
pub use resource_lookup::ImageResource;
pub use resource_lookup::BufferKey;
pub use resource_lookup::RenderPassResource;
pub use resource_lookup::FramebufferResource;
pub use resource_lookup::MaterialPassResource;
pub use resource_lookup::ShaderModuleResource;
pub use resource_lookup::BufferResource;
pub use resource_lookup::PipelineLayoutResource;
pub use resource_lookup::GraphicsPipelineResource;
pub use resource_lookup::ImageViewResource;

mod dyn_resource_allocator;
pub use dyn_resource_allocator::DynResourceAllocatorSet;
pub use dyn_resource_allocator::DynResourceAllocatorSetProvider;

pub mod descriptor_sets;
pub use descriptor_sets::DescriptorSetAllocator;
pub use descriptor_sets::DescriptorSetAllocatorRef;
pub use descriptor_sets::DescriptorSetAllocatorProvider;
pub use descriptor_sets::DescriptorSetArc;
pub use descriptor_sets::DescriptorSetAllocatorMetrics;
pub use descriptor_sets::DynDescriptorSet;
pub use descriptor_sets::DescriptorSetWriteSet;

mod resource_manager;
pub use resource_manager::*;

mod command_buffers;
pub use command_buffers::CommandPool;
pub use command_buffers::DynCommandWriterAllocator;
pub use command_buffers::DynCommandWriter;

mod pipeline_cache;
pub use pipeline_cache::GraphicsPipelineCache;
