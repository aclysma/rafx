// Not compiling rust, but should communicate what the API looks like!

//
// Root of the API
//

/// Primary entry point to using the API. Use the new_* functions to initialize the desired backend.
pub struct RafxApi;
impl RafxApi {
    pub fn new(window: &dyn HasRawWindowHandle, api_def: &RafxApiDef) -> RafxResult<Self>;
    pub fn new_vulkan(window: &dyn HasRawWindowHandle,api_def: &RafxApiDef, vk_api_def: &RafxApiDefVulkan) -> RafxResult<Self>;
    pub fn new_metal(window: &dyn HasRawWindowHandle, api_def: &RafxApiDef, vk_api_def: &RafxApiDefMetal) -> RafxResult<Self>;

    pub fn device_context(&self);

    pub fn destroy(&mut self);
}

/// A cloneable, thread-safe handle used to create graphics resources.
#[derive(Clone)]
pub struct RafxDeviceContext;
impl RafxDeviceContext {
    pub fn device_info(&self) -> &RafxDeviceInfo;

    pub fn create_queue(&self, queue_type: RafxQueueType) -> RafxResult<RafxQueue>;
    pub fn create_fence(&self) -> RafxResult<RafxFence>;
    pub fn create_semaphore(&self) -> RafxResult<RafxSemaphore>;
    pub fn create_swapchain(&self, raw_window_handle: &dyn HasRawWindowHandle, swapchain_def: &RafxSwapchainDef) -> RafxResult<RafxSwapchain>;
    pub fn create_sampler(&self, sampler_def: &RafxSamplerDef) -> RafxResult<RafxSampler>;
    pub fn create_texture(&self, texture_def: &RafxTextureDef) -> RafxResult<RafxTexture>;
    pub fn create_buffer(&self, buffer_def: &RafxBufferDef) -> RafxResult<RafxBuffer>;
    pub fn create_shader(&self, stages: Vec<RafxShaderStageDef>) -> RafxResult<RafxShader>;
    pub fn create_root_signature(&self, root_signature_def: &RafxRootSignatureDef) -> RafxResult<RafxRootSignature>;
    pub fn create_descriptor_set_array(&self, descriptor_set_array_def: &RafxDescriptorSetArrayDef) -> RafxResult<RafxDescriptorSetArray>;
    pub fn create_graphics_pipeline(&self, graphics_pipeline_def: &RafxGraphicsPipelineDef) -> RafxResult<RafxPipeline>;
    pub fn create_compute_pipeline(&self, compute_pipeline_def: &RafxComputePipelineDef) -> RafxResult<RafxPipeline>;
    pub fn create_shader_module(&self, data: RafxShaderModuleDef) -> RafxResult<RafxShaderModule>;

    pub fn wait_for_fences(&self, fences: &[&RafxFence]) -> RafxResult<()>;

    pub fn find_supported_format(&self, candidates: &[RafxFormat], resource_type: RafxResourceType) -> Option<RafxFormat>;
    pub fn find_supported_sample_count(&self, candidates: &[RafxSampleCount]) -> Option<RafxSampleCount>;
}

//
// Resources (Buffers, Textures, Samplers)
//

/// A buffer is a piece of memory that can be accessed by the GPU. It may reside in CPU or GPU memory.
#[derive(Debug)]
pub struct RafxBuffer;
impl RafxBuffer {
    pub fn buffer_def(&self) -> &RafxBufferDef;
    pub fn map_buffer(&self) -> RafxResult<*mut u8>;
    pub fn unmap_buffer(&self) -> RafxResult<()>;
    pub fn copy_to_host_visible_buffer<T: Copy>(&self, data: &[T]) -> RafxResult<()>;
    pub fn copy_to_host_visible_buffer_with_offset<T: Copy>(
        &self, data: &[T], buffer_byte_offset: u64) -> RafxResult<()>;
}

/// An image that can be used by the GPU.
#[derive(Debug)]
pub struct RafxTexture;
impl RafxTexture {
    pub fn texture_def(&self) -> &RafxTextureDef;
}

/// Configures how images will be sampled by the GPU
#[derive(Clone, Debug)]
pub struct RafxSampler;

//
// Shaders/Pipelines
//

/// Rrepresents loaded shader code that can be used to create a pipeline.
#[derive(Clone, Debug)]
pub struct RafxShaderModule;

/// Represents one or more shader stages, producing an entire "program" to execute on the GPU
#[derive(Clone, Debug)]
pub struct RafxShader;
impl RafxShader {
    pub fn pipeline_reflection(&self) -> &RafxPipelineReflection;
}

/// Represents the full "layout" or "interface" of a shader (or set of shaders.)
#[derive(Clone, Debug)]
pub struct RafxRootSignature;
impl RafxRootSignature {
    pub fn pipeline_type(&self) -> RafxPipelineType;
}

/// Represents a complete GPU configuration for executing work.
#[derive(Debug)]
pub struct RafxPipeline;
impl RafxPipeline {
    pub fn pipeline_type(&self) -> RafxPipelineType;
    pub fn root_signature(&self) -> &RafxRootSignature;
}

//
// Descriptor Sets
//

/// Represents an array of descriptor sets.
#[derive(Debug)]
pub struct RafxDescriptorSetArray;
impl RafxDescriptorSetArray {
    pub fn handle(&self, array_index: u32) -> Option<RafxDescriptorSetHandle>;
    pub fn root_signature(&self) -> &RafxRootSignature;
    pub fn update_descriptor_set(&mut self, params: &[RafxDescriptorUpdate]) -> RafxResult<()>;
    pub fn queue_descriptor_set_update(&mut self, update: &RafxDescriptorUpdate) -> RafxResult<()>;
    pub fn flush_descriptor_set_updates(&mut self) -> RafxResult<()>;
}

/// A lightweight handle to a specific descriptor set in a RafxDescriptorSetArray.
#[derive(Clone, Debug)]
pub struct RafxDescriptorSetHandle;

//
// Queues, Command Buffers
//

/// A queue allows work to be submitted to the GPU
#[derive(Clone, Debug)]
pub struct RafxQueue;
impl RafxQueue {
    pub fn queue_id(&self) -> u32;
    pub fn queue_type(&self) -> RafxQueueType;
    pub fn create_command_pool(&self, command_pool_def: &RafxCommandPoolDef) -> RafxResult<RafxCommandPool>;
    pub fn submit(&self, command_buffers: &[&RafxCommandBuffer], wait_semaphores: &[&RafxSemaphore], signal_semaphores: &[&RafxSemaphore], signal_fence: Option<&RafxFence>) -> RafxResult<()>;
    pub fn present(&self, swapchain: &RafxSwapchain, wait_semaphores: &[&RafxSemaphore], image_index: u32) -> RafxResult<RafxPresentSuccessResult>;
    pub fn wait_for_queue_idle(&self) -> RafxResult<()>;
}

/// A pool of command buffers. All command buffers must be created from a pool.
pub struct RafxCommandPool;
impl RafxCommandPool {
    pub fn device_context(&self) -> &RafxDeviceContext;
    pub fn create_command_buffer(&self, command_buffer_def: &RafxCommandBufferDef) -> RafxResult<RafxCommandBuffer>;
    pub fn reset_command_pool(&self) -> RafxResult<()>;
}

/// A command buffer contains a list of work for the GPU to do.
#[derive(Debug)]
pub struct RafxCommandBuffer;
impl RafxCommandBuffer {
    pub fn begin(&self) -> RafxResult<()>;
    pub fn end(&self) -> RafxResult<()>;
    pub fn return_to_pool(&self) -> RafxResult<()>;

    pub fn cmd_begin_render_pass(&self, color_targets: &[RafxColorRenderTargetBinding], depth_target: Option<RafxDepthRenderTargetBinding>) -> RafxResult<()>;
    pub fn cmd_end_render_pass(&self) -> RafxResult<()>;

    pub fn cmd_set_viewport(&self, x: f32, y: f32, width: f32, height: f32, depth_min: f32, depth_max: f32) -> RafxResult<()>;
    pub fn cmd_set_scissor(&self, x: u32, y: u32, width: u32, height: u32) -> RafxResult<()>;
    pub fn cmd_set_stencil_reference_value(&self, value: u32) -> RafxResult<()>;

    pub fn cmd_bind_pipeline(&self, pipeline: &RafxPipeline) -> RafxResult<()>;
    pub fn cmd_bind_vertex_buffers(&self, first_binding: u32, bindings: &[RafxVertexBufferBinding]) -> RafxResult<()>;
    pub fn cmd_bind_index_buffer(&self, binding: &RafxIndexBufferBinding) -> RafxResult<()>;
    pub fn cmd_bind_descriptor_set(&self, descriptor_set_array: &RafxDescriptorSetArray, index: u32) -> RafxResult<()>;
    pub fn cmd_bind_descriptor_set_handle(&self, root_signature: &RafxRootSignature, set_index: u32, descriptor_set_handle: &RafxDescriptorSetHandle) -> RafxResult<()>;

    pub fn cmd_draw(&self, vertex_count: u32, first_vertex: u32) -> RafxResult<()>;
    pub fn cmd_draw_instanced(&self, vertex_count: u32, first_vertex: u32, instance_count: u32, first_instance: u32) -> RafxResult<()>;
    pub fn cmd_draw_indexed(&self, index_count: u32, first_index: u32, vertex_offset: i32) -> RafxResult<()>;
    pub fn cmd_draw_indexed_instanced(&self, index_count: u32,  first_index: u32,  instance_count: u32,  first_instance: u32,vertex_offset: i32) -> RafxResult<()>;

    pub fn cmd_dispatch(&self, group_count_x: u32,  group_count_y: u32, group_count_z: u32) -> RafxResult<()>;

    pub fn cmd_resource_barrier(&self, buffer_barriers: &[RafxBufferBarrier], texture_barriers: &[RafxTextureBarrier], render_target_barriers: &[RafxRenderTargetBarrier]) -> RafxResult<()>;
    pub fn cmd_copy_buffer_to_buffer(&self, src_buffer: &RafxBuffer, dst_buffer: &RafxBuffer, src_offset: u64, dst_offset: u64, size: u64) -> RafxResult<()>;
    pub fn cmd_copy_buffer_to_texture(&self, src_buffer: &RafxBuffer, dst_texture: &RafxTexture, params: &RafxCmdCopyBufferToTextureParams) -> RafxResult<()>;
}

//
// Fences and Semaphores
//

/// A GPU -> CPU synchronization mechanism.
pub struct RafxFence;
impl RafxFence {
    pub fn wait(&self) -> RafxResult<()>;
    pub fn wait_for_fences(device_context: &RafxDeviceContext, fences: &[&RafxFence]) -> RafxResult<()>;
    pub fn get_fence_status(&self) -> RafxResult<RafxFenceStatus>;
}

/// A GPU -> GPU synchronization mechanism.
pub struct RafxSemaphore;

//
// Swapchain
//

/// A set of images that act as a "backbuffer" of a window.
pub struct RafxSwapchain;
impl RafxSwapchain {
    pub fn swapchain_def(&self) -> &RafxSwapchainDef;
    pub fn image_count(&self) -> usize;
    pub fn format(&self) -> RafxFormat;
    pub fn acquire_next_image_fence(&mut self, fence: &RafxFence) -> RafxResult<RafxSwapchainImage>;
    pub fn acquire_next_image_semaphore(&mut self, semaphore: &RafxSemaphore) -> RafxResult<RafxSwapchainImage>;
    pub fn rebuild(&mut self, swapchain_def: &RafxSwapchainDef) -> RafxResult<()>;
}
