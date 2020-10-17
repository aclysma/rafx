mod resource_arc;
use resource_arc::ResourceId;
pub use resource_arc::ResourceArc;

mod resource_lookup;
pub use resource_lookup::ResourceLookupSet;
pub use resource_lookup::ResourceHash;
pub use resource_lookup::DescriptorSetLayoutResource;
pub use resource_lookup::ImageKey;
pub use resource_lookup::BufferKey;
pub use resource_lookup::RenderPassResource;
pub use resource_lookup::FramebufferResource;

mod dyn_resource_allocator;
pub use dyn_resource_allocator::DynResourceAllocatorSet;

mod load_queue;
pub use load_queue::LoadQueues;
pub use load_queue::GenericLoader;

mod swapchain_management;

mod asset_lookup;
pub use asset_lookup::AssetLookupSet;
pub use asset_lookup::AssetLookup;

mod descriptor_sets;
pub use descriptor_sets::DescriptorSetAllocatorRef;
pub use descriptor_sets::DescriptorSetAllocatorProvider;
pub use descriptor_sets::DescriptorSetArc;
pub use descriptor_sets::DescriptorSetAllocatorMetrics;
pub use descriptor_sets::DynDescriptorSet;
pub use descriptor_sets::DynPassMaterialInstance;
pub use descriptor_sets::DynMaterialInstance;
pub use descriptor_sets::DescriptorSetWriteSet;

mod upload;
pub use crate::resources::resource_lookup::PipelineLayoutResource;
pub use crate::resources::resource_lookup::PipelineResource;

pub use resource_lookup::ImageViewResource;

mod pipeline_create_data;
pub use pipeline_create_data::PipelineCreateData;

mod resource_manager;
pub use resource_manager::*;
