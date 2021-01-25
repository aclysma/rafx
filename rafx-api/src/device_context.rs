#[cfg(feature = "rafx-metal")]
use crate::metal::RafxDeviceContextMetal;
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::RafxDeviceContextVulkan;
use crate::*;
use raw_window_handle::HasRawWindowHandle;

/// A cloneable, thread-safe handle used to create graphics resources.
///
/// All device contexts, and resources created from them, must be dropped before the `RafxApi`
/// object that they came from is dropped or destroyed.
#[derive(Clone)]
pub enum RafxDeviceContext {
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxDeviceContextVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxDeviceContextMetal),
}

impl RafxDeviceContext {
    /// Get metadata about the device
    pub fn device_info(&self) -> &RafxDeviceInfo {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => inner.device_info(),
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => inner.device_info(),
        }
    }

    pub fn find_supported_format(
        &self,
        candidates: &[RafxFormat],
        resource_type: RafxResourceType,
    ) -> Option<RafxFormat> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => inner.find_supported_format(candidates, resource_type),
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => {
                inner.find_supported_format(candidates, resource_type)
            }
        }
    }

    /// Create a queue
    pub fn create_queue(
        &self,
        queue_type: RafxQueueType,
    ) -> RafxResult<RafxQueue> {
        Ok(match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => RafxQueue::Vk(inner.create_queue(queue_type)?),
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => RafxQueue::Metal(inner.create_queue(queue_type)?),
        })
    }

    /// Create a fence
    pub fn create_fence(&self) -> RafxResult<RafxFence> {
        Ok(match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => RafxFence::Vk(inner.create_fence()?),
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => RafxFence::Metal(inner.create_fence()?),
        })
    }

    /// Create a semaphore
    pub fn create_semaphore(&self) -> RafxResult<RafxSemaphore> {
        Ok(match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => RafxSemaphore::Vk(inner.create_semaphore()?),
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => RafxSemaphore::Metal(inner.create_semaphore()?),
        })
    }

    /// Create a swapchain
    pub fn create_swapchain(
        &self,
        raw_window_handle: &dyn HasRawWindowHandle,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<RafxSwapchain> {
        Ok(match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => {
                RafxSwapchain::Vk(inner.create_swapchain(raw_window_handle, swapchain_def)?)
            }
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => {
                RafxSwapchain::Metal(inner.create_swapchain(raw_window_handle, swapchain_def)?)
            }
        })
    }

    /// Wait for the given fences to complete. If a fence is in an unsubmitted state, the fence is
    /// ignored.
    pub fn wait_for_fences(
        &self,
        fences: &[&RafxFence],
    ) -> RafxResult<()> {
        Ok(match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => {
                let fences: Vec<_> = fences.iter().map(|x| x.vk_fence().unwrap()).collect();
                inner.wait_for_fences(&fences)?
            }
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => {
                let fences: Vec<_> = fences.iter().map(|x| x.metal_fence().unwrap()).collect();
                inner.wait_for_fences(&fences)?
            }
        })
    }

    /// Create a sampler
    pub fn create_sampler(
        &self,
        sampler_def: &RafxSamplerDef,
    ) -> RafxResult<RafxSampler> {
        Ok(match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => RafxSampler::Vk(inner.create_sampler(sampler_def)?),
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => {
                RafxSampler::Metal(inner.create_sampler(sampler_def)?)
            }
        })
    }

    /// Create a texture
    pub fn create_texture(
        &self,
        texture_def: &RafxTextureDef,
    ) -> RafxResult<RafxTexture> {
        Ok(match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => RafxTexture::Vk(inner.create_texture(texture_def)?),
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => {
                RafxTexture::Metal(inner.create_texture(texture_def)?)
            }
        })
    }

    /// Create a render target
    pub fn create_render_target(
        &self,
        render_target_def: &RafxRenderTargetDef,
    ) -> RafxResult<RafxRenderTarget> {
        Ok(match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => {
                RafxRenderTarget::Vk(inner.create_render_target(render_target_def)?)
            }
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => {
                RafxRenderTarget::Metal(inner.create_render_target(render_target_def)?)
            }
        })
    }

    /// Create a buffer
    pub fn create_buffer(
        &self,
        buffer_def: &RafxBufferDef,
    ) -> RafxResult<RafxBuffer> {
        Ok(match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => RafxBuffer::Vk(inner.create_buffer(buffer_def)?),
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => RafxBuffer::Metal(inner.create_buffer(buffer_def)?),
        })
    }

    pub fn create_shader_module(
        &self,
        shader_module_def: RafxShaderModuleDef,
    ) -> RafxResult<RafxShaderModule> {
        Ok(match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => {
                RafxShaderModule::Vk(inner.create_shader_module(shader_module_def.vk.unwrap())?)
            }
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => RafxShaderModule::Metal(
                inner.create_shader_module(shader_module_def.metal.unwrap())?,
            ),
        })
    }

    //TODO: Consider a struct with each kind of shader stage instead of a vec of stages
    /// Create a shader
    pub fn create_shader(
        &self,
        stages: Vec<RafxShaderStageDef>,
    ) -> RafxResult<RafxShader> {
        Ok(match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => RafxShader::Vk(inner.create_shader(stages)?),
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => RafxShader::Metal(inner.create_shader(stages)?),
        })
    }

    /// Create a root signature
    pub fn create_root_signature(
        &self,
        root_signature_def: &RafxRootSignatureDef,
    ) -> RafxResult<RafxRootSignature> {
        Ok(match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => {
                RafxRootSignature::Vk(inner.create_root_signature(root_signature_def)?)
            }
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => {
                RafxRootSignature::Metal(inner.create_root_signature(root_signature_def)?)
            }
        })
    }

    /// Create a graphics pipeline
    pub fn create_graphics_pipeline(
        &self,
        pipeline_def: &RafxGraphicsPipelineDef,
    ) -> RafxResult<RafxPipeline> {
        Ok(match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => {
                RafxPipeline::Vk(inner.create_graphics_pipeline(pipeline_def)?)
            }
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => {
                RafxPipeline::Metal(inner.create_graphics_pipeline(pipeline_def)?)
            }
        })
    }

    /// Create a compute pipeline
    pub fn create_compute_pipeline(
        &self,
        pipeline_def: &RafxComputePipelineDef,
    ) -> RafxResult<RafxPipeline> {
        Ok(match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => {
                RafxPipeline::Vk(inner.create_compute_pipeline(pipeline_def)?)
            }
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => {
                RafxPipeline::Metal(inner.create_compute_pipeline(pipeline_def)?)
            }
        })
    }

    /// Create a descriptor set array
    pub fn create_descriptor_set_array(
        &self,
        descriptor_set_array_def: &RafxDescriptorSetArrayDef,
    ) -> RafxResult<RafxDescriptorSetArray> {
        Ok(match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => RafxDescriptorSetArray::Vk(
                inner.create_descriptor_set_array(descriptor_set_array_def)?,
            ),
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => RafxDescriptorSetArray::Metal(
                inner.create_descriptor_set_array(descriptor_set_array_def)?,
            ),
        })
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_device_context(&self) -> Option<&RafxDeviceContextVulkan> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_device_context(&self) -> Option<&RafxDeviceContextMetal> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => Some(inner),
        }
    }
}
