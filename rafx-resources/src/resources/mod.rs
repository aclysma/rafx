mod resource_arc;
pub use resource_arc::ResourceArc;
pub(crate) use resource_arc::ResourceId;

mod resource_lookup;
pub use resource_lookup::BufferKey;
pub use resource_lookup::BufferResource;
pub use resource_lookup::ComputePipelineKey;
pub use resource_lookup::ComputePipelineResource;
pub use resource_lookup::DescriptorSetLayoutResource;
pub use resource_lookup::FixedFunctionState;
pub use resource_lookup::GraphicsPipelineResource;
pub use resource_lookup::ImageKey;
pub use resource_lookup::ImageResource;
pub use resource_lookup::ImageViewResource;
pub use resource_lookup::MaterialPassResource;
pub use resource_lookup::MaterialPassVertexInput;
pub use resource_lookup::ResourceHash;
pub use resource_lookup::ResourceLookupSet;
pub use resource_lookup::SamplerResource;
pub use resource_lookup::ShaderModuleHash;
pub use resource_lookup::ShaderModuleMeta;
pub use resource_lookup::ShaderModuleResource;
pub use resource_lookup::ShaderModuleResourceDef;

mod dyn_resources;
pub use dyn_resources::DynResourceAllocatorSet;
pub use dyn_resources::DynResourceAllocatorSetProvider;

pub mod descriptor_sets;
pub use descriptor_sets::DescriptorSetAllocator;
pub use descriptor_sets::DescriptorSetAllocatorMetrics;
pub use descriptor_sets::DescriptorSetAllocatorProvider;
pub use descriptor_sets::DescriptorSetAllocatorRef;
pub use descriptor_sets::DescriptorSetArc;
pub use descriptor_sets::DescriptorSetInitializer;
pub use descriptor_sets::DescriptorSetLayout;
pub use descriptor_sets::DescriptorSetLayoutBinding;
pub use descriptor_sets::DescriptorSetWriteSet;
pub use descriptor_sets::DynDescriptorSet;

mod resource_manager;
pub use resource_manager::*;

mod dyn_commands;
pub use dyn_commands::DynCommandBuffer;
pub use dyn_commands::DynCommandPool;
pub use dyn_commands::DynCommandPoolAllocator;

mod pipeline_cache;
pub use pipeline_cache::GraphicsPipelineCache;
pub use pipeline_cache::GraphicsPipelineRenderTargetMeta;
pub use pipeline_cache::GraphicsPipelineRenderTargetMetaHash;

mod vertex_data;
pub use vertex_data::VertexCopyError;
pub use vertex_data::VertexData;
pub use vertex_data::VertexDataLayout;
pub use vertex_data::VertexDataLayoutHash;
pub use vertex_data::VertexDataSet;
pub use vertex_data::VertexDataSetLayout;
pub use vertex_data::VertexMember;

mod pool;
pub use pool::DescriptorSetArrayPoolAllocator;
pub use pool::PooledResourceAllocator;
pub use pool::PooledResourceImpl;

mod cleanup;
pub use cleanup::ResourceDropSink;

pub mod cooked_shader;
pub use cooked_shader::*;

pub use rafx_base::resource_map::ResourceMap as RenderResources;
