use crate::{
    RafxApiDefMetal, RafxBufferDef, RafxComputePipelineDef, RafxDescriptorSetArrayDef,
    RafxDeviceContext, RafxDeviceInfo, RafxFormat, RafxGraphicsPipelineDef, RafxQueueType,
    RafxResourceType, RafxResult, RafxRootSignatureDef, RafxSampleCount, RafxSamplerDef,
    RafxShaderModuleDefMetal, RafxShaderStageDef, RafxSwapchainDef, RafxTextureDef,
};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::sync::Arc;

use crate::metal::features::MetalFeatures;
use crate::metal::{
    RafxBufferMetal, RafxDescriptorSetArrayMetal, RafxFenceMetal, RafxPipelineMetal,
    RafxQueueMetal, RafxRootSignatureMetal, RafxSamplerMetal, RafxSemaphoreMetal, RafxShaderMetal,
    RafxShaderModuleMetal, RafxSwapchainMetal, RafxTextureMetal,
};

#[cfg(debug_assertions)]
#[cfg(feature = "track-device-contexts")]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct RafxDeviceContextMetalInner {
    pub(crate) device_info: RafxDeviceInfo,

    device: metal_rs::Device,
    destroyed: AtomicBool,

    #[cfg(debug_assertions)]
    #[cfg(feature = "track-device-contexts")]
    next_create_index: AtomicU64,
    metal_features: MetalFeatures,

    #[cfg(debug_assertions)]
    #[cfg(feature = "track-device-contexts")]
    pub(crate) all_contexts: Mutex<fnv::FnvHashMap<u64, backtrace::Backtrace>>,
}

// For metal_rs::Device
unsafe impl Send for RafxDeviceContextMetalInner {}
unsafe impl Sync for RafxDeviceContextMetalInner {}

impl Drop for RafxDeviceContextMetalInner {
    fn drop(&mut self) {
        log::trace!("destroying device");
        self.destroyed.swap(true, Ordering::AcqRel);
    }
}

impl RafxDeviceContextMetalInner {
    pub fn new(metal_api_def: &RafxApiDefMetal) -> RafxResult<Self> {
        let device_info = RafxDeviceInfo {
            supports_multithreaded_usage: true,
            debug_names_enabled: metal_api_def.enable_debug_names,
            // pretty sure this is consistent across macOS device (maybe not M1, not sure)
            min_uniform_buffer_offset_alignment: 256,
            // based on one of the loosest vulkan limits (intel iGPU), can't find official value
            min_storage_buffer_offset_alignment: 64,
            upload_texture_alignment: 16,
            upload_texture_row_alignment: 1,
            supports_clamp_to_border_color: true, //TODO: Check for iOS support
            max_vertex_attribute_count: 31,
        };

        #[cfg(debug_assertions)]
        #[cfg(feature = "track-device-contexts")]
        let all_contexts = {
            let create_backtrace = backtrace::Backtrace::new_unresolved();
            let mut all_contexts = fnv::FnvHashMap::<u64, backtrace::Backtrace>::default();
            all_contexts.insert(0, create_backtrace);
            all_contexts
        };

        let device = metal_rs::Device::system_default().expect("no device found");

        let metal_features = MetalFeatures::from_device(device.as_ref());

        log::debug!("Device: {}", metal_features.device_name);
        log::debug!(
            "GPU family (apple={:?} mac={:?} common={:?}",
            metal_features.gpu_family_apple,
            metal_features.gpu_family_mac,
            metal_features.gpu_family_common
        );

        Ok(RafxDeviceContextMetalInner {
            device_info,
            device,
            metal_features,
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
    pub fn device_info(&self) -> &RafxDeviceInfo {
        &self.inner.device_info
    }

    pub fn device(&self) -> &metal_rs::Device {
        &self.inner.device
    }

    pub fn metal_features(&self) -> &MetalFeatures {
        &self.inner.metal_features
    }

    pub fn new(inner: Arc<RafxDeviceContextMetalInner>) -> RafxResult<Self> {
        Ok(RafxDeviceContextMetal {
            inner,
            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            create_index: 0,
        })
    }

    pub fn create_queue(
        &self,
        queue_type: RafxQueueType,
    ) -> RafxResult<RafxQueueMetal> {
        RafxQueueMetal::new(self, queue_type)
    }

    pub fn create_fence(&self) -> RafxResult<RafxFenceMetal> {
        RafxFenceMetal::new(self)
    }

    pub fn create_semaphore(&self) -> RafxResult<RafxSemaphoreMetal> {
        RafxSemaphoreMetal::new(self)
    }

    pub fn create_swapchain(
        &self,
        raw_display_handle: &dyn HasRawDisplayHandle,
        raw_window_handle: &dyn HasRawWindowHandle,
        _present_queue: &RafxQueueMetal,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<RafxSwapchainMetal> {
        RafxSwapchainMetal::new(self, raw_display_handle, raw_window_handle, swapchain_def)
    }

    pub fn wait_for_fences(
        &self,
        fences: &[&RafxFenceMetal],
    ) -> RafxResult<()> {
        RafxFenceMetal::wait_for_fences(self, fences)
    }

    pub fn create_sampler(
        &self,
        sampler_def: &RafxSamplerDef,
    ) -> RafxResult<RafxSamplerMetal> {
        RafxSamplerMetal::new(self, sampler_def)
    }

    pub fn create_texture(
        &self,
        texture_def: &RafxTextureDef,
    ) -> RafxResult<RafxTextureMetal> {
        RafxTextureMetal::new(self, texture_def)
    }

    pub fn create_buffer(
        &self,
        buffer_def: &RafxBufferDef,
    ) -> RafxResult<RafxBufferMetal> {
        RafxBufferMetal::new(self, buffer_def)
    }

    pub fn create_shader(
        &self,
        stages: Vec<RafxShaderStageDef>,
    ) -> RafxResult<RafxShaderMetal> {
        RafxShaderMetal::new(self, stages)
    }

    pub fn create_root_signature(
        &self,
        root_signature_def: &RafxRootSignatureDef,
    ) -> RafxResult<RafxRootSignatureMetal> {
        RafxRootSignatureMetal::new(self, root_signature_def)
    }

    pub fn create_descriptor_set_array(
        &self,
        descriptor_set_array_def: &RafxDescriptorSetArrayDef,
    ) -> RafxResult<RafxDescriptorSetArrayMetal> {
        RafxDescriptorSetArrayMetal::new(self, descriptor_set_array_def)
    }

    pub fn create_graphics_pipeline(
        &self,
        graphics_pipeline_def: &RafxGraphicsPipelineDef,
    ) -> RafxResult<RafxPipelineMetal> {
        RafxPipelineMetal::new_graphics_pipeline(self, graphics_pipeline_def)
    }

    pub fn create_compute_pipeline(
        &self,
        compute_pipeline_def: &RafxComputePipelineDef,
    ) -> RafxResult<RafxPipelineMetal> {
        RafxPipelineMetal::new_compute_pipeline(self, compute_pipeline_def)
    }

    pub fn create_shader_module(
        &self,
        data: RafxShaderModuleDefMetal,
    ) -> RafxResult<RafxShaderModuleMetal> {
        RafxShaderModuleMetal::new(self, data)
    }

    pub fn find_supported_format(
        &self,
        candidates: &[RafxFormat],
        resource_type: RafxResourceType,
    ) -> Option<RafxFormat> {
        // https://developer.apple.com/metal/Metal-Feature-Set-Tables.pdf
        use metal_rs::PixelFormatCapabilities;

        let mut required_capabilities = PixelFormatCapabilities::empty();

        // Some formats include color and not write, so I think it's not necessary to have write
        // capability for color attachments
        if resource_type.intersects(RafxResourceType::RENDER_TARGET_COLOR) {
            required_capabilities |= PixelFormatCapabilities::Color;
        }

        // Depth formats don't include write, so presumably it's implied that a depth format can
        // be a depth attachment
        // if resource_type.intersects(RafxResourceType::RENDER_TARGET_DEPTH_STENCIL) {
        //     required_capabilities |= PixelFormatCapabilities::Write;
        // }

        if resource_type.intersects(RafxResourceType::TEXTURE_READ_WRITE) {
            required_capabilities |= PixelFormatCapabilities::Write;
        }

        for &candidate in candidates {
            let capabilities = self
                .inner
                .metal_features
                .pixel_format_capabilities(candidate.into());
            if capabilities.contains(required_capabilities) {
                return Some(candidate);
            }
        }

        None
    }

    pub fn find_supported_sample_count(
        &self,
        candidates: &[RafxSampleCount],
    ) -> Option<RafxSampleCount> {
        for &candidate in candidates {
            if self
                .inner
                .device
                .supports_texture_sample_count(candidate.into())
            {
                return Some(candidate);
            }
        }

        None
    }
}
