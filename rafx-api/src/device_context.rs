#[cfg(any(
    feature = "rafx-empty",
    not(any(
        feature = "rafx-metal",
        feature = "rafx-vulkan",
        feature = "rafx-gles2",
        feature = "rafx-gles3",
    ))
))]
use crate::backends::empty::RafxDeviceContextEmpty;
#[cfg(feature = "rafx-gles2")]
use crate::gles2::RafxDeviceContextGles2;
#[cfg(feature = "rafx-gles3")]
use crate::gles3::RafxDeviceContextGles3;
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
    #[cfg(feature = "rafx-gles2")]
    Gles2(RafxDeviceContextGles2),
    #[cfg(feature = "rafx-gles3")]
    Gles3(RafxDeviceContextGles3),
    #[cfg(any(
        feature = "rafx-empty",
        not(any(
            feature = "rafx-metal",
            feature = "rafx-vulkan",
            feature = "rafx-gles2",
            feature = "rafx-gles3",
        ))
    ))]
    Empty(RafxDeviceContextEmpty),
}

impl RafxDeviceContext {
    pub fn is_vulkan(&self) -> bool {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(_) => true,
            _ => false,
        }
    }

    pub fn is_metal(&self) -> bool {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(_) => true,
            _ => false,
        }
    }

    pub fn is_gles3(&self) -> bool {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(_) => true,
            _ => false,
        }
    }

    pub fn is_gles2(&self) -> bool {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(_) => true,
            _ => false,
        }
    }

    /// Get metadata about the device
    pub fn device_info(&self) -> &RafxDeviceInfo {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => inner.device_info(),
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => inner.device_info(),
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(inner) => inner.device_info(),
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(inner) => inner.device_info(),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxDeviceContext::Empty(inner) => inner.device_info(),
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
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(inner) => {
                inner.find_supported_format(candidates, resource_type)
            }
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(inner) => {
                inner.find_supported_format(candidates, resource_type)
            }
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxDeviceContext::Empty(inner) => {
                inner.find_supported_format(candidates, resource_type)
            }
        }
    }

    pub fn find_supported_sample_count(
        &self,
        candidates: &[RafxSampleCount],
    ) -> Option<RafxSampleCount> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => inner.find_supported_sample_count(candidates),
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => inner.find_supported_sample_count(candidates),
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(inner) => inner.find_supported_sample_count(candidates),
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(inner) => inner.find_supported_sample_count(candidates),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3",
                ))
            ))]
            RafxDeviceContext::Empty(inner) => inner.find_supported_sample_count(candidates),
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
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(inner) => RafxQueue::Gles2(inner.create_queue(queue_type)?),
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(inner) => RafxQueue::Gles3(inner.create_queue(queue_type)?),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3",
                ))
            ))]
            RafxDeviceContext::Empty(inner) => RafxQueue::Empty(inner.create_queue(queue_type)?),
        })
    }

    /// Create a fence
    pub fn create_fence(&self) -> RafxResult<RafxFence> {
        Ok(match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => RafxFence::Vk(inner.create_fence()?),
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => RafxFence::Metal(inner.create_fence()?),
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(inner) => RafxFence::Gles2(inner.create_fence()?),
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(inner) => RafxFence::Gles3(inner.create_fence()?),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxDeviceContext::Empty(inner) => RafxFence::Empty(inner.create_fence()?),
        })
    }

    /// Create a semaphore
    pub fn create_semaphore(&self) -> RafxResult<RafxSemaphore> {
        Ok(match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(inner) => RafxSemaphore::Vk(inner.create_semaphore()?),
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(inner) => RafxSemaphore::Metal(inner.create_semaphore()?),
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(inner) => RafxSemaphore::Gles2(inner.create_semaphore()?),
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(inner) => RafxSemaphore::Gles3(inner.create_semaphore()?),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxDeviceContext::Empty(inner) => RafxSemaphore::Empty(inner.create_semaphore()?),
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
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(inner) => {
                RafxSwapchain::Gles2(inner.create_swapchain(raw_window_handle, swapchain_def)?)
            }
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(inner) => {
                RafxSwapchain::Gles3(inner.create_swapchain(raw_window_handle, swapchain_def)?)
            }
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3",
                ))
            ))]
            RafxDeviceContext::Empty(inner) => {
                RafxSwapchain::Empty(inner.create_swapchain(raw_window_handle, swapchain_def)?)
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
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(inner) => {
                let fences: Vec<_> = fences.iter().map(|x| x.gles2_fence().unwrap()).collect();
                inner.wait_for_fences(&fences)?
            }
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(inner) => {
                let fences: Vec<_> = fences.iter().map(|x| x.gles3_fence().unwrap()).collect();
                inner.wait_for_fences(&fences)?
            }
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxDeviceContext::Empty(inner) => {
                let fences: Vec<_> = fences.iter().map(|x| x.empty_fence().unwrap()).collect();
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
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(inner) => {
                RafxSampler::Gles2(inner.create_sampler(sampler_def)?)
            }
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(inner) => {
                RafxSampler::Gles3(inner.create_sampler(sampler_def)?)
            }
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxDeviceContext::Empty(inner) => {
                RafxSampler::Empty(inner.create_sampler(sampler_def)?)
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
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(inner) => {
                RafxTexture::Gles2(inner.create_texture(texture_def)?)
            }
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(inner) => {
                RafxTexture::Gles3(inner.create_texture(texture_def)?)
            }
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxDeviceContext::Empty(inner) => {
                RafxTexture::Empty(inner.create_texture(texture_def)?)
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
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(inner) => RafxBuffer::Gles2(inner.create_buffer(buffer_def)?),
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(inner) => RafxBuffer::Gles3(inner.create_buffer(buffer_def)?),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxDeviceContext::Empty(inner) => RafxBuffer::Empty(inner.create_buffer(buffer_def)?),
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
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(inner) => RafxShaderModule::Gles2(
                inner.create_shader_module(shader_module_def.gles2.unwrap())?,
            ),
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(inner) => RafxShaderModule::Gles3(
                inner.create_shader_module(shader_module_def.gles3.unwrap())?,
            ),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3",
                ))
            ))]
            RafxDeviceContext::Empty(inner) => RafxShaderModule::Empty(
                inner.create_shader_module(shader_module_def.empty.unwrap())?,
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
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(inner) => RafxShader::Gles2(inner.create_shader(stages)?),
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(inner) => RafxShader::Gles3(inner.create_shader(stages)?),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3",
                ))
            ))]
            RafxDeviceContext::Empty(inner) => RafxShader::Empty(inner.create_shader(stages)?),
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
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(inner) => {
                RafxRootSignature::Gles2(inner.create_root_signature(root_signature_def)?)
            }
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(inner) => {
                RafxRootSignature::Gles3(inner.create_root_signature(root_signature_def)?)
            }
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxDeviceContext::Empty(inner) => {
                RafxRootSignature::Empty(inner.create_root_signature(root_signature_def)?)
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
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(inner) => {
                RafxPipeline::Gles2(inner.create_graphics_pipeline(pipeline_def)?)
            }
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(inner) => {
                RafxPipeline::Gles3(inner.create_graphics_pipeline(pipeline_def)?)
            }
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxDeviceContext::Empty(inner) => {
                RafxPipeline::Empty(inner.create_graphics_pipeline(pipeline_def)?)
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
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(inner) => {
                RafxPipeline::Gles2(inner.create_compute_pipeline(pipeline_def)?)
            }
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(inner) => {
                RafxPipeline::Gles3(inner.create_compute_pipeline(pipeline_def)?)
            }
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxDeviceContext::Empty(inner) => {
                RafxPipeline::Empty(inner.create_compute_pipeline(pipeline_def)?)
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
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(inner) => RafxDescriptorSetArray::Gles2(
                inner.create_descriptor_set_array(descriptor_set_array_def)?,
            ),
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(inner) => RafxDescriptorSetArray::Gles3(
                inner.create_descriptor_set_array(descriptor_set_array_def)?,
            ),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxDeviceContext::Empty(inner) => RafxDescriptorSetArray::Empty(
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
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxDeviceContext::Empty(_) => None,
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
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxDeviceContext::Empty(_) => None,
        }
    }

    /// Get the underlying gl API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gles2")]
    pub fn gles2_device_context(&self) -> Option<&RafxDeviceContextGles2> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(inner) => Some(inner),
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxDeviceContext::Empty(_) => None,
        }
    }

    /// Get the underlying gl API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gles3")]
    pub fn gles3_device_context(&self) -> Option<&RafxDeviceContextGles3> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(inner) => Some(inner),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxDeviceContext::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(any(
        feature = "rafx-empty",
        not(any(
            feature = "rafx-metal",
            feature = "rafx-vulkan",
            feature = "rafx-gles2",
            feature = "rafx-gles3"
        ))
    ))]
    pub fn empty_device_context(&self) -> Option<&RafxDeviceContextEmpty> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDeviceContext::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxDeviceContext::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxDeviceContext::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxDeviceContext::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxDeviceContext::Empty(inner) => Some(inner),
        }
    }
}
