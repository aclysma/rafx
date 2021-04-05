//! Rafx API is an `unsafe` graphics API abstraction layer designed specifically for games and tools
//! for games. The goal is to achieve near-native performance with reduced complexity. It may be
//! used directly, or indirectly through other crates in rafx (such as [rafx-resources] and
//! [rafx-assets]).
//!
//! `rafx-api` is an **opinionated** API. It does not expose every possible operation a graphics API
//! might provide. However, the wrapped API-specific objects are exposed in an easily accessible
//! manner.
//!
//! The API does not track resource lifetimes or states (such as vulkan image layouts) or try to
//! enforce safe usage at compile time or runtime. Safer abstractions are available in
//! rafx-framework and rafx-assets.
//!
//! **Every API call is potentially unsafe.** However, the unsafe keyword is only placed on APIs
//! that are particularly likely to cause undefined behavior if used incorrectly.
//!
//! The general shape of the API is inspired by
//! [The Forge](https://github.com/ConfettiFX/The-Forge). It was chosen for its modern design,
//! multiple working backends, open development model, and track record of shipped games. However,
//! there are some changes in API design, feature set, and implementation details.
//!
//! [Additional high-level documentation is available in the github repo](https://github.com/aclysma/rafx/blob/master/docs/index.md)
//!
//! # Main API Objects
//!
//! * [RafxApi] - Primary entry point to using the API. Use the new_* functions to initialize the desired backend.
//! * [RafxBuffer] - Memory that can be accessed by the rendering API. It may reside in CPU or GPU memory.
//! * [RafxCommandBuffer] - A list of commands recorded by the CPU and submitted to the GPU.
//! * [RafxCommandPool] - A pool of command buffers. A command pool is necessary to create a command buffer.
//! * [RafxDescriptorSetArray] - An array of descriptor sets. These are expected to be pooled and reused.
//! * [RafxDeviceContext] - A cloneable, thread-safe handle used to create graphics resources.
//! * [RafxFence] - A GPU -> CPU synchronization mechanism.
//! * [RafxPipeline] - Represents a complete GPU configuration for executing work.
//! * [RafxQueue] - A queue allows work to be submitted to the GPU
//! * [RafxRootSignature] - Represents the full "layout" or "interface" of a shader (or set of shaders.)
//! * [RafxSampler] - Configures how images will be sampled by the GPU
//! * [RafxSemaphore] - A GPU -> GPU synchronization mechanism.
//! * [RafxShader] - Represents one or more shader stages, producing an entire "program" to execute on the GPU
//! * [RafxShaderModule] - Rrepresents loaded shader code that can be used to create a pipeline.
//! * [RafxSwapchain] - A set of images that act as a "backbuffer" of a window.
//! * [RafxTexture] - An image that can be used by the GPU.
//!
//! # Usage Summary
//!
//! In order to interact with a graphics API, construct a `RafxApi`. A different new_* function
//! exists for each backend.
//!
//! ```ignore
//! let api = RafxApi::new_vulkan(...);
//! ```
//!
//! After initialization, most interaction will be via `RafxDeviceContext` Call
//! `RafxApi::device_context()` on the the api object to obtain a cloneable handle that can be
//! used from multiple threads.
//!
//! ```ignore
//! let device_context = api.device_context();
//! ```
//!
//! Most objects are created via `RafxDeviceContext`. For example:
//!
//! ```ignore
//! // (See examples for more detail here!)
//! let texture = device_context.create_texture(...)?;
//! let buffer = device_context.create_buffer(...)?;
//! let shader_module = device_context.create_shader_module(...)?;
//! ```
//!
//! In order to submit work to the GPU, a `RafxCommandBuffer` must be submitted to a `RafxQueue`.
//! Most commonly, this needs to be a "Graphics" queue.
//!
//! Obtaining a `RafxQueue` is straightforward. Here we will get a "Graphics" queue. This queue type
//! supports ALL operations (including compute) and is usually the correct one to use if you aren't
//! sure.
//!
//! ```ignore
//! let queue = device_context.create_queue(RafxQueueType::Graphics)?;
//! ```
//!
//! A command buffer cannot be created directly. It must be allocated out of a pool.
//!
//! The command pool and all command buffers allocated from it share memory. The standard rust rules
//! about mutability apply but are not enforced at compile time or runtime.
//!  * Do not modify two command buffers from the same pool concurrently
//!  * Do not allocate from a command pool while modifying one of its command buffers
//!  * Once a command buffer is submitted to the GPU, do not modify its pool, or any command buffers
//!    created from it, until the GPU completes its work.
//!
//! In general, do not modify textures, buffers, command buffers, or other GPU resources while a
//! command buffer referencing them is submitted. Additionally, these resources must persist for
//! the entire duration of the submitted workload.
//!
//! ```ignore
//! let command_pool = queue.create_command_pool(&RafxCommandPoolDef {
//!     transient: true
//! })?;
//!
//! let command_buffer = command_pool.create_command_buffer(&RafxCommandBufferDef {
//!     is_secondary: false,
//! })?;
//! ```
//!
//! Once a command buffer is obtained, write to it by calling "cmd" functions on it, For example,
//! drawing primitives looks like this. Call begin() before writing to it, and end() after finished
//! writing to it.
//!
//! ```ignore
//! command_buffer.begin()?;
//! // other setup...
//! command_buffer.cmd_draw(3, 0)?;
//! command_buffer.end()?;
//! ```
//!
//! For the most part, no actual work is performed when calling these functions. We are just
//! "scheduling" work to happen later when we give the command buffer to the GPU.
//!
//! After writing the command buffer, it must be submitted to the queue. The "scheduled" work
//! described in the command buffer will happen asynchronously from the rest of the program.
//!
//! ```ignore
//! queue.submit(
//!     &[&command_buffer],
//!     &[], // No semaphores or fences in this example
//!     &[],
//!     None
//! )?;
//! queue.wait_for_queue_idle()?;
//! ```
//!
//! The command buffer, the command pool it was allocated from, all other command buffers allocated
//! from that pool, and any other resources referenced by this command buffer cannot be dropped
//! until the queued work is complete, and generally speaking must remain immutable.
//!
//! More fine-grained synchronization is available via RafxFence and RafxSemaphore but that will
//! not be covered here.
//!
//! # Resource Barriers
//!
//! CPUs generally provide a single "coherent" view of memory, but this is not the case for GPUs.
//! Resources can also be stored in many forms depending on how they are used. (The details of this
//! are device-specific and outside the scope of these docs). Resources must be placed into an
//! appropriate state to use them.
//!
//! Additionally modifying a resource (or transitioning its state) can result in memory hazards. A
//! memory hazard is when reading/writing to memory occurs in an undefined order, resulting in
//! undefined behavior.
//!
//! `Barriers` are used to transition resources into the correct state and to avoid these hazards.
//! Here is an example where we take an image from the swapchain and prepare it for use.
//! (We will also need a barrier after we modify it to transition it back to PRESENT!)
//!
//! ```ignore
//! command_buffer.cmd_resource_barrier(
//!     &[], // no buffers to transition
//!     &[
//!         // Transition `texture` from PRESENT state to RENDER_TARGET state
//!         RafxTextureBarrier::state_transition(
//!             &texture,
//!             RafxResourceState::PRESENT,
//!             RafxResourceState::RENDER_TARGET,
//!         )
//!     ],
//! )?;
//! ```
//!
//! # "Definition" structs
//!
//! Many functions take a "def" parameter. For example, `RafxDeviceContext::create_texture()` takes
//! a single `RafxTextureDef` parameter. Here is an example call:
//!
//! ```ignore
//!     let texture = device_context.create_texture(&RafxTextureDef {
//!         extents: RafxExtents3D {
//!             width: 512,
//!             height: 512,
//!             depth: 1,
//!         },
//!         array_length: 1,
//!         mip_count: 1,
//!         sample_count: RafxSampleCount::SampleCount1,
//!         format: RafxFormat::R8G8B8A8_UNORM,
//!         resource_type: RafxResourceType::TEXTURE,
//!         dimensions: RafxTextureDimensions::Dim2D,
//!     })?;
//! ```
//!
//! There are advantages to this approach:
//! * The code is easier to read - parameters are clearly labeled
//! * Default values can be used
//! * When new "parameters" are added, if Default is used, the code will still compile. This avoids
//!   boilerplate to implement the builder pattern
//!
//! ```ignore
//!     let texture = device_context.create_texture(&RafxTextureDef {
//!         extents: RafxExtents3D {
//!             width: 512,
//!             height: 512,
//!             depth: 1,
//!         },
//!         format: RafxFormat::R8G8B8A8_UNORM,
//!         ..Default::default()
//!     })?;
//! ```
//!
//!

//
// Re-export upstream metal crates
//
#[cfg(feature = "rafx-metal")]
pub use foreign_types_shared;
#[cfg(feature = "rafx-metal")]
pub use metal_rs;
#[cfg(feature = "rafx-metal")]
pub use objc;

//
// Re-export upstream vulkan crates
//
#[cfg(feature = "rafx-vulkan")]
pub use ash;
#[cfg(feature = "rafx-vulkan")]
pub use vk_mem;

//
// Re-export upstream API-agnostic crates
//
pub use raw_window_handle;

//
// Backends
//
#[cfg(feature = "rafx-metal")]
pub use backends::metal;
#[cfg(feature = "rafx-metal")]
pub use backends::metal::RafxApiDefMetal;

#[cfg(feature = "rafx-vulkan")]
pub use backends::vulkan;
#[cfg(feature = "rafx-vulkan")]
pub use backends::vulkan::RafxApiDefVulkan;

#[cfg(feature = "rafx-gl")]
pub use backends::gl;
#[cfg(feature = "rafx-gl")]
pub use backends::gl::RafxApiDefGl;

#[cfg(any(
    feature = "rafx-empty",
    not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
))]
pub use backends::empty;

//
// Public modules
//
pub mod extra;

//
// Internal Modules
//
mod backends;
mod error;
mod internal_shared;
mod reflection;
mod types;

//
// API-agnostic API modules
//
mod api;
mod buffer;
mod command_buffer;
mod command_pool;
mod descriptor_set_array;
mod device_context;
mod fence;
mod pipeline;
mod queue;
mod root_signature;
mod sampler;
mod semaphore;
mod shader;
mod shader_module;
mod swapchain;
mod texture;

//
// Constants
//

/// The maximum descriptor set layout index allowed. Vulkan only guarantees up to 4 are available
pub const MAX_DESCRIPTOR_SET_LAYOUTS: usize = 4;
/// The maximum number of simultaneously attached render targets
// In sync with RafxBlendStateTargets
pub const MAX_RENDER_TARGET_ATTACHMENTS: usize = 8;

//
// Exported public API
//
pub use api::*;
pub use buffer::*;
pub use command_buffer::*;
pub use command_pool::*;
pub use descriptor_set_array::*;
pub use device_context::*;
pub use error::*;
pub use extra::swapchain_helper::*;
pub use fence::*;
pub use pipeline::*;
pub use queue::*;
pub use root_signature::*;
pub use sampler::*;
pub use semaphore::*;
pub use shader::*;
pub use shader_module::*;
pub use swapchain::*;
pub use texture::*;
pub use types::*;
