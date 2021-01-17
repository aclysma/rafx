use crate::{
    RafxBufferDef, RafxComputePipelineDef, RafxDescriptorSetArrayDef, RafxDeviceContext,
    RafxDeviceInfo, RafxFormat, RafxGraphicsPipelineDef, RafxQueueType, RafxRenderTargetDef,
    RafxResourceType, RafxResult, RafxRootSignatureDef, RafxSampleCount, RafxSamplerDef,
    RafxShaderModule, RafxShaderModuleDef, RafxShaderModuleDefMetal, RafxShaderStageDef,
    RafxSwapchainDef, RafxTextureDef,
};
use raw_window_handle::HasRawWindowHandle;
use std::sync::{Arc, Mutex};

// use crate::metal::{
//     RafxBufferMetal, RafxDescriptorSetArrayMetal, RafxFenceMetal, RafxPipelineMetal,
//     RafxQueueMetal, RafxRenderTargetMetal, RafxRootSignatureMetal, RafxSamplerMetal,
//     RafxSemaphoreMetal, RafxShaderModuleMetal, RafxShaderMetal, RafxSwapchainMetal,
//     RafxTextureMetal,
// };
use fnv::FnvHashMap;
#[cfg(debug_assertions)]
#[cfg(feature = "track-device-contexts")]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct RafxDeviceContextMetalInner {
    //pub(crate) device_info: RafxDeviceInfo,

    //device: ash::Device,
    destroyed: AtomicBool,

    #[cfg(debug_assertions)]
    #[cfg(feature = "track-device-contexts")]
    next_create_index: AtomicU64,

    #[cfg(debug_assertions)]
    #[cfg(feature = "track-device-contexts")]
    pub(crate) all_contexts: Mutex<fnv::FnvHashMap<u64, backtrace::Backtrace>>,
}

impl Drop for RafxDeviceContextMetalInner {
    fn drop(&mut self) {
        if !self.destroyed.swap(true, Ordering::AcqRel) {
            unsafe {
                log::trace!("destroying device");

                log::trace!("destroyed device");
            }
        }
    }
}

impl RafxDeviceContextMetalInner {
    pub fn new() -> RafxResult<Self> {
        // let device_info = RafxDeviceInfo {
        //     min_uniform_buffer_offset_alignment: limits.min_uniform_buffer_offset_alignment as u32,
        //     min_storage_buffer_offset_alignment: limits.min_storage_buffer_offset_alignment as u32,
        //     upload_buffer_texture_alignment: limits.optimal_buffer_copy_offset_alignment as u32,
        //     upload_buffer_texture_row_alignment: limits.optimal_buffer_copy_row_pitch_alignment
        //         as u32,
        // };

        #[cfg(debug_assertions)]
        #[cfg(feature = "track-device-contexts")]
        let all_contexts = {
            let create_backtrace = backtrace::Backtrace::new_unresolved();
            let mut all_contexts = fnv::FnvHashMap::<u64, backtrace::Backtrace>::default();
            all_contexts.insert(0, create_backtrace);
            all_contexts
        };

        Ok(RafxDeviceContextMetalInner {
            //device_info,
            destroyed: AtomicBool::new(false),

            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            all_contexts: Mutex::new(all_contexts),

            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            next_create_index: AtomicU64::new(1),
        })
    }
}

pub struct RafxDeviceContextMetal {
    pub(crate) inner: Arc<RafxDeviceContextMetalInner>,
    #[cfg(debug_assertions)]
    #[cfg(feature = "track-device-contexts")]
    pub(crate) create_index: u64,
}

impl std::fmt::Debug for RafxDeviceContextMetal {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        f.debug_struct("RafxDeviceContextMetal")
            //.field("handle", &self.device().handle())
            .finish()
    }
}

impl Clone for RafxDeviceContextMetal {
    fn clone(&self) -> Self {
        #[cfg(debug_assertions)]
        #[cfg(feature = "track-device-contexts")]
        let create_index = {
            let create_index = self.inner.next_create_index.fetch_add(1, Ordering::Relaxed);

            #[cfg(feature = "track-device-contexts")]
            {
                let create_backtrace = backtrace::Backtrace::new_unresolved();
                self.inner
                    .as_ref()
                    .all_contexts
                    .lock()
                    .unwrap()
                    .insert(create_index, create_backtrace);
            }

            log::trace!(
                "Cloned RafxDeviceContextMetal create_index {}",
                create_index
            );
            create_index
        };

        RafxDeviceContextMetal {
            inner: self.inner.clone(),
            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            create_index,
        }
    }
}

impl Drop for RafxDeviceContextMetal {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        #[cfg(feature = "track-device-contexts")]
        {
            self.inner
                .all_contexts
                .lock()
                .unwrap()
                .remove(&self.create_index);
        }
    }
}

impl Into<RafxDeviceContext> for RafxDeviceContextMetal {
    fn into(self) -> RafxDeviceContext {
        RafxDeviceContext::Metal(self)
    }
}

impl RafxDeviceContextMetal {
    // pub fn device_info(&self) -> &RafxDeviceInfo {
    //     &self.inner.device_info
    // }

    pub fn new(inner: Arc<RafxDeviceContextMetalInner>) -> RafxResult<Self> {
        Ok(RafxDeviceContextMetal {
            inner,
            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            create_index: 0,
        })
    }
    //
    // pub fn create_queue(
    //     &self,
    //     queue_type: RafxQueueType,
    // ) -> RafxResult<RafxQueueMetal> {
    //     RafxQueueMetal::new(self, queue_type)
    // }
    //
    // pub fn create_fence(&self) -> RafxResult<RafxFenceMetal> {
    //     RafxFenceMetal::new(self)
    // }
    //
    // pub fn create_semaphore(&self) -> RafxResult<RafxSemaphoreMetal> {
    //     RafxSemaphoreMetal::new(self)
    // }
    //
    // pub fn create_swapchain(
    //     &self,
    //     raw_window_handle: &dyn HasRawWindowHandle,
    //     swapchain_def: &RafxSwapchainDef,
    // ) -> RafxResult<RafxSwapchainMetal> {
    //     RafxSwapchainMetal::new(self, raw_window_handle, swapchain_def)
    // }
    //
    // pub fn wait_for_fences(
    //     &self,
    //     fences: &[&RafxFenceMetal],
    // ) -> RafxResult<()> {
    //     let mut fence_list = Vec::with_capacity(fences.len());
    //     for fence in fences {
    //         if fence.submitted() {
    //             fence_list.push(fence.vk_fence());
    //         }
    //     }
    //
    //     if !fence_list.is_empty() {
    //         let device = self.device();
    //         unsafe {
    //             device.wait_for_fences(&fence_list, true, std::u64::MAX)?;
    //             device.reset_fences(&fence_list)?;
    //         }
    //     }
    //
    //     for fence in fences {
    //         fence.set_submitted(false);
    //     }
    //
    //     Ok(())
    // }
    //
    // pub fn wait_for_device_idle(&self) -> RafxResult<()> {
    //     unsafe {
    //         self.device().device_wait_idle()?;
    //         Ok(())
    //     }
    // }
    //
    // pub fn create_sampler(
    //     &self,
    //     sampler_def: &RafxSamplerDef,
    // ) -> RafxResult<RafxSamplerMetal> {
    //     RafxSamplerMetal::new(self, sampler_def)
    // }
    //
    // pub fn create_texture(
    //     &self,
    //     texture_def: &RafxTextureDef,
    // ) -> RafxResult<RafxTextureMetal> {
    //     RafxTextureMetal::new(self, texture_def)
    // }
    //
    // pub fn create_render_target(
    //     &self,
    //     render_target_def: &RafxRenderTargetDef,
    // ) -> RafxResult<RafxRenderTargetMetal> {
    //     RafxRenderTargetMetal::new(self, render_target_def)
    // }
    //
    // pub fn create_buffer(
    //     &self,
    //     buffer_def: &RafxBufferDef,
    // ) -> RafxResult<RafxBufferMetal> {
    //     RafxBufferMetal::new(self, buffer_def)
    // }
    //
    // pub fn create_shader(
    //     &self,
    //     stages: Vec<RafxShaderStageDef>,
    // ) -> RafxResult<RafxShaderMetal> {
    //     RafxShaderMetal::new(self, stages)
    // }
    //
    // pub fn create_root_signature(
    //     &self,
    //     root_signature_def: &RafxRootSignatureDef,
    // ) -> RafxResult<RafxRootSignatureMetal> {
    //     RafxRootSignatureMetal::new(self, root_signature_def)
    // }
    //
    // pub fn create_descriptor_set_array(
    //     &self,
    //     descriptor_set_array_def: &RafxDescriptorSetArrayDef,
    // ) -> RafxResult<RafxDescriptorSetArrayMetal> {
    //     RafxDescriptorSetArrayMetal::new(self, self.descriptor_heap(), descriptor_set_array_def)
    // }
    //
    // pub fn create_graphics_pipeline(
    //     &self,
    //     graphics_pipeline_def: &RafxGraphicsPipelineDef,
    // ) -> RafxResult<RafxPipelineMetal> {
    //     RafxPipelineMetal::new_graphics_pipeline(self, graphics_pipeline_def)
    // }
    //
    // pub fn create_compute_pipeline(
    //     &self,
    //     compute_pipeline_def: &RafxComputePipelineDef,
    // ) -> RafxResult<RafxPipelineMetal> {
    //     RafxPipelineMetal::new_compute_pipeline(self, compute_pipeline_def)
    // }
    //
    // pub(crate) fn create_renderpass(
    //     &self,
    //     renderpass_def: &RafxRenderpassMetalDef,
    // ) -> RafxResult<RafxRenderpassMetal> {
    //     RafxRenderpassMetal::new(self, renderpass_def)
    // }
    //
    // pub fn create_shader_module(
    //     &self,
    //     data: RafxShaderModuleDefMetal
    // ) -> RafxResult<RafxShaderModuleMetal> {
    //     match data {
    //         RafxShaderModuleDefMetal::VkSpvBytes(bytes) => RafxShaderModuleMetal::new_from_bytes(self, bytes),
    //         RafxShaderModuleDefMetal::VkSpvPrepared(spv) => RafxShaderModuleMetal::new_from_spv(self, spv),
    //     }
    // }
    //
    // pub fn find_supported_format(
    //     &self,
    //     candidates: &[RafxFormat],
    //     resource_type: RafxResourceType,
    // ) -> Option<RafxFormat> {
    //     let mut features = vk::FormatFeatureFlags::empty();
    //     if resource_type.intersects(RafxResourceType::RENDER_TARGET_COLOR) {
    //         features |= vk::FormatFeatureFlags::COLOR_ATTACHMENT;
    //     }
    //
    //     if resource_type.intersects(RafxResourceType::RENDER_TARGET_DEPTH_STENCIL) {
    //         features |= vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT;
    //     }
    //
    //     do_find_supported_format(
    //         &self.inner.instance,
    //         self.inner.physical_device,
    //         candidates,
    //         vk::ImageTiling::OPTIMAL,
    //         features,
    //     )
    // }
    //
    // pub fn find_supported_sample_count(
    //     &self,
    //     candidates: &[RafxSampleCount],
    // ) -> Option<RafxSampleCount> {
    //     do_find_supported_sample_count(self.limits(), candidates)
    // }
}
