use ash::vk;

mod api;
pub use api::*;

mod device_context;
pub use device_context::*;

mod swapchain;
pub use swapchain::*;

mod shader_module;
pub use shader_module::*;

mod shader;
pub use shader::*;

mod queue;
pub use queue::*;

mod command_pool;
pub use command_pool::*;

mod command_buffer;
pub use command_buffer::*;

mod fence;
pub use fence::*;

mod semaphore;
pub use semaphore::*;

mod texture;
pub use texture::*;

mod render_target;
pub use render_target::*;

mod buffer;
pub use buffer::*;

mod root_signature;
pub use root_signature::*;

mod pipeline;
pub use pipeline::*;

mod sampler;
pub use sampler::*;

mod descriptor_set_array;
pub use descriptor_set_array::*;

mod internal;
pub(crate) use internal::*;

// If using an SDR format, consider using the swapchain surface format!
pub const DEFAULT_COLOR_FORMATS_SDR: [vk::Format; 1] = [
    vk::Format::R8G8B8A8_SNORM, // 100% coverage with optimal
];

pub const DEFAULT_COLOR_FORMATS_HDR: [vk::Format; 1] = [
    vk::Format::R32G32B32A32_SFLOAT, // 100% coverage with optimal
];

pub const DEFAULT_DEPTH_FORMATS: [vk::Format; 3] = [
    vk::Format::D32_SFLOAT,         // 100% coverage with optimal
    vk::Format::D32_SFLOAT_S8_UINT, // 100% coverage with optimal
    vk::Format::D24_UNORM_S8_UINT,
];

pub use internal::util::resource_type_to_descriptor_type;
