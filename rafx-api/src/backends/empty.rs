// Don't use standard formatting in this file
#![rustfmt::skip]
#![allow(unused_attributes)]
#![allow(unused_variables)]

use crate::*;
use raw_window_handle::HasRawWindowHandle;

//
// Root of the API
//
pub struct RafxApiEmpty;
impl RafxApiEmpty {
    pub fn device_context(&self) -> &RafxDeviceContextEmpty { unimplemented!() }

    pub fn destroy(&mut self) -> RafxResult<()> { unimplemented!() }
}

#[derive(Clone)]
pub struct RafxDeviceContextEmpty;
impl RafxDeviceContextEmpty {
    pub fn device_info(&self) -> &RafxDeviceInfo { unimplemented!() }

    pub fn create_queue(&self, queue_type: RafxQueueType) -> RafxResult<RafxQueueEmpty> { unimplemented!(); }
    pub fn create_fence(&self) -> RafxResult<RafxFenceEmpty> { unimplemented!(); }
    pub fn create_semaphore(&self) -> RafxResult<RafxSemaphoreEmpty> { unimplemented!(); }
    pub fn create_swapchain(&self, raw_window_handle: &dyn HasRawWindowHandle, swapchain_def: &RafxSwapchainDef) -> RafxResult<RafxSwapchainEmpty> { unimplemented!(); }
    pub fn create_sampler(&self, sampler_def: &RafxSamplerDef) -> RafxResult<RafxSamplerEmpty> { unimplemented!(); }
    pub fn create_texture(&self, texture_def: &RafxTextureDef) -> RafxResult<RafxTextureEmpty> { unimplemented!(); }
    pub fn create_buffer(&self, buffer_def: &RafxBufferDef) -> RafxResult<RafxBufferEmpty> { unimplemented!(); }
    pub fn create_shader(&self, stages: Vec<RafxShaderStageDef>) -> RafxResult<RafxShaderEmpty> { unimplemented!(); }
    pub fn create_root_signature(&self, root_signature_def: &RafxRootSignatureDef) -> RafxResult<RafxRootSignatureEmpty> { unimplemented!(); }
    pub fn create_descriptor_set_array(&self, descriptor_set_array_def: &RafxDescriptorSetArrayDef) -> RafxResult<RafxDescriptorSetArrayEmpty> { unimplemented!(); }
    pub fn create_graphics_pipeline(&self, graphics_pipeline_def: &RafxGraphicsPipelineDef) -> RafxResult<RafxPipelineEmpty> { unimplemented!(); }
    pub fn create_compute_pipeline(&self, compute_pipeline_def: &RafxComputePipelineDef) -> RafxResult<RafxPipelineEmpty> { unimplemented!(); }
    pub fn create_shader_module(&self, data: RafxShaderModuleDefEmpty) -> RafxResult<RafxShaderModuleEmpty> { unimplemented!(); }

    pub fn wait_for_fences(&self, fences: &[&RafxFenceEmpty]) -> RafxResult<()> { unimplemented!(); }

    pub fn find_supported_format(&self, candidates: &[RafxFormat], resource_type: RafxResourceType) -> Option<RafxFormat> { unimplemented!(); }
    pub fn find_supported_sample_count(&self, candidates: &[RafxSampleCount]) -> Option<RafxSampleCount> { unimplemented!(); }
}

//
// Resources (Buffers, Textures, Samplers)
//
#[derive(Debug)]
pub struct RafxBufferEmpty;
impl RafxBufferEmpty {
    pub fn buffer_def(&self) -> &RafxBufferDef { unimplemented!() }
    pub fn map_buffer(&self) -> RafxResult<*mut u8> { unimplemented!() }
    pub fn unmap_buffer(&self) -> RafxResult<()> { unimplemented!() }
    pub fn mapped_memory(&self) -> Option<*mut u8> { unimplemented!() }
    pub fn copy_to_host_visible_buffer<T: Copy>(&self, data: &[T]) -> RafxResult<()> { unimplemented!() }
    pub fn copy_to_host_visible_buffer_with_offset<T: Copy>(
        &self, data: &[T], buffer_byte_offset: u64) -> RafxResult<()> { unimplemented!() }
}

#[derive(Clone, Debug)]
pub struct RafxTextureEmpty;
impl RafxTextureEmpty {
    pub fn texture_def(&self) -> &RafxTextureDef { unimplemented!() }
}

#[derive(Clone, Debug)]
pub struct RafxSamplerEmpty;

//
// Shaders/Pipelines
//
#[derive(Clone, Debug)]
pub struct RafxShaderModuleEmpty;

#[derive(Clone, Debug)]
pub struct RafxShaderEmpty;
impl RafxShaderEmpty {
    pub fn pipeline_reflection(&self) -> &RafxPipelineReflection { unimplemented!() }
}

#[derive(Clone, Debug)]
pub struct RafxRootSignatureEmpty;
impl RafxRootSignatureEmpty {
    pub fn pipeline_type(&self) -> RafxPipelineType { unimplemented!() }
}

#[derive(Debug)]
pub struct RafxPipelineEmpty;
impl RafxPipelineEmpty {
    pub fn pipeline_type(&self) -> RafxPipelineType { unimplemented!(); }
    pub fn root_signature(&self) -> &RafxRootSignature { unimplemented!(); }
}

//
// Descriptor Sets
//
#[derive(Clone, Debug)]
pub struct RafxDescriptorSetHandleEmpty;

#[derive(Debug)]
pub struct RafxDescriptorSetArrayEmpty;
impl RafxDescriptorSetArrayEmpty {
    pub fn handle(&self, array_index: u32) -> Option<RafxDescriptorSetHandleEmpty> { unimplemented!(); }
    pub fn root_signature(&self) -> &RafxRootSignature { unimplemented!(); }
    pub fn update_descriptor_set(&mut self, params: &[RafxDescriptorUpdate]) -> RafxResult<()> { unimplemented!(); }
    pub fn queue_descriptor_set_update(&mut self, update: &RafxDescriptorUpdate) -> RafxResult<()> { unimplemented!(); }
    pub fn flush_descriptor_set_updates(&mut self) -> RafxResult<()> { unimplemented!(); }
}

//
// Queues, Command Buffers
//
#[derive(Clone, Debug)]
pub struct RafxQueueEmpty;
impl RafxQueueEmpty {
    pub fn device_context(&self) -> &RafxDeviceContextEmpty { unimplemented!() }
    pub fn queue_id(&self) -> u32 { unimplemented!(); }
    pub fn queue_type(&self) -> RafxQueueType { unimplemented!(); }
    pub fn create_command_pool(&self, command_pool_def: &RafxCommandPoolDef) -> RafxResult<RafxCommandPoolEmpty> { unimplemented!(); }
    pub fn submit(&self, command_buffers: &[&RafxCommandBufferEmpty], wait_semaphores: &[&RafxSemaphoreEmpty], signal_semaphores: &[&RafxSemaphoreEmpty], signal_fence: Option<&RafxFenceEmpty>) -> RafxResult<()> { unimplemented!(); }
    pub fn present(&self, swapchain: &RafxSwapchainEmpty, wait_semaphores: &[&RafxSemaphoreEmpty], image_index: u32) -> RafxResult<RafxPresentSuccessResult> { unimplemented!() }
    pub fn wait_for_queue_idle(&self) -> RafxResult<()> { unimplemented!() }
}

pub struct RafxCommandPoolEmpty;
impl RafxCommandPoolEmpty {
    pub fn device_context(&self) -> &RafxDeviceContextEmpty { unimplemented!() }
    pub fn create_command_buffer(&self, command_buffer_def: &RafxCommandBufferDef) -> RafxResult<RafxCommandBufferEmpty> { unimplemented!() }
    pub fn reset_command_pool(&self) -> RafxResult<()> { unimplemented!() }
}

#[derive(Debug)]
pub struct RafxCommandBufferEmpty;
impl RafxCommandBufferEmpty {
    pub fn begin(&self) -> RafxResult<()> { unimplemented!() }
    pub fn end(&self) -> RafxResult<()> { unimplemented!() }
    pub fn return_to_pool(&self) -> RafxResult<()> { unimplemented!() }

    pub fn cmd_begin_render_pass(&self, color_targets: &[RafxColorRenderTargetBinding], depth_target: Option<RafxDepthStencilRenderTargetBinding>) -> RafxResult<()> { unimplemented!() }
    pub fn cmd_end_render_pass(&self) -> RafxResult<()> { unimplemented!() }

    pub fn cmd_set_viewport(&self, x: f32, y: f32, width: f32, height: f32, depth_min: f32, depth_max: f32) -> RafxResult<()> { unimplemented!() }
    pub fn cmd_set_scissor(&self, x: u32, y: u32, width: u32, height: u32) -> RafxResult<()> { unimplemented!() }
    pub fn cmd_set_stencil_reference_value(&self, value: u32) -> RafxResult<()> { unimplemented!() }

    pub fn cmd_bind_pipeline(&self, pipeline: &RafxPipelineEmpty) -> RafxResult<()> { unimplemented!() }
    pub fn cmd_bind_vertex_buffers(&self, first_binding: u32, bindings: &[RafxVertexBufferBinding]) -> RafxResult<()> { unimplemented!() }
    pub fn cmd_bind_index_buffer(&self, binding: &RafxIndexBufferBinding) -> RafxResult<()> { unimplemented!() }
    pub fn cmd_bind_descriptor_set(&self, descriptor_set_array: &RafxDescriptorSetArrayEmpty, index: u32) -> RafxResult<()> { unimplemented!() }
    pub fn cmd_bind_descriptor_set_handle(&self, root_signature: &RafxRootSignatureEmpty, set_index: u32, descriptor_set_handle: &RafxDescriptorSetHandleEmpty) -> RafxResult<()> { unimplemented!() }

    pub fn cmd_draw(&self, vertex_count: u32, first_vertex: u32) -> RafxResult<()> { unimplemented!() }
    pub fn cmd_draw_instanced(&self, vertex_count: u32, first_vertex: u32, instance_count: u32, first_instance: u32) -> RafxResult<()> { unimplemented!() }
    pub fn cmd_draw_indexed(&self, index_count: u32, first_index: u32, vertex_offset: i32) -> RafxResult<()> { unimplemented!() }
    pub fn cmd_draw_indexed_instanced(&self, index_count: u32,  first_index: u32,  instance_count: u32,  first_instance: u32,vertex_offset: i32) -> RafxResult<()> { unimplemented!() }

    pub fn cmd_dispatch(&self, group_count_x: u32,  group_count_y: u32, group_count_z: u32) -> RafxResult<()> { unimplemented!() }

    pub fn cmd_resource_barrier(&self, buffer_barriers: &[RafxBufferBarrier], texture_barriers: &[RafxTextureBarrier]) -> RafxResult<()> { unimplemented!() }
    pub fn cmd_copy_buffer_to_buffer(&self, src_buffer: &RafxBufferEmpty, dst_buffer: &RafxBufferEmpty, src_offset: u64, dst_offset: u64, size: u64) -> RafxResult<()> { unimplemented!() }
    pub fn cmd_copy_buffer_to_texture(&self, src_buffer: &RafxBufferEmpty, dst_texture: &RafxTextureEmpty, params: &RafxCmdCopyBufferToTextureParams) -> RafxResult<()> { unimplemented!() }
}

//
// Fences and Semaphores
//
pub struct RafxFenceEmpty;
impl RafxFenceEmpty {
    pub fn wait(&self) -> RafxResult<()> { unimplemented!(); }
    pub fn wait_for_fences(device_context: &RafxDeviceContextEmpty, fences: &[&RafxFenceEmpty]) -> RafxResult<()> { unimplemented!(); }
    pub fn get_fence_status(&self) -> RafxResult<RafxFenceStatus> { unimplemented!(); }
}

pub struct RafxSemaphoreEmpty;

//
// Swapchain
//
pub struct RafxSwapchainEmpty;
impl RafxSwapchainEmpty {
    pub fn swapchain_def(&self) -> &RafxSwapchainDef { unimplemented!() }
    pub fn image_count(&self) -> usize { unimplemented!() }
    pub fn format(&self) -> RafxFormat { unimplemented!() }
    pub fn acquire_next_image_fence(&mut self, fence: &RafxFenceEmpty) -> RafxResult<RafxSwapchainImage> { unimplemented!() }
    pub fn acquire_next_image_semaphore(&mut self, semaphore: &RafxSemaphoreEmpty) -> RafxResult<RafxSwapchainImage> { unimplemented!() }
    pub fn rebuild(&mut self, swapchain_def: &RafxSwapchainDef) -> RafxResult<()> { unimplemented!() }
}
