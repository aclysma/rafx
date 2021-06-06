use crate::{
    RafxApiDefGles3, RafxBufferDef, RafxComputePipelineDef, RafxDescriptorSetArrayDef,
    RafxDeviceContext, RafxDeviceInfo, RafxFormat, RafxGraphicsPipelineDef, RafxQueueType,
    RafxResourceType, RafxResult, RafxRootSignatureDef, RafxSampleCount, RafxSamplerDef,
    RafxShaderModuleDefGles3, RafxShaderStageDef, RafxSwapchainDef, RafxTextureDef,
};
use raw_window_handle::HasRawWindowHandle;
use std::sync::Arc;

use crate::gles3::{
    GlContextManager, RafxBufferGles3, RafxDescriptorSetArrayGles3, RafxFenceGles3,
    RafxPipelineGles3, RafxQueueGles3, RafxRootSignatureGles3, RafxSamplerGles3,
    RafxSemaphoreGles3, RafxShaderGles3, RafxShaderModuleGles3, RafxSwapchainGles3,
    RafxTextureGles3,
};

use crate::gles3::gles3_bindings;
use crate::gles3::GlContext;

use crate::gles3::fullscreen_quad::FullscreenQuad;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub struct RafxDeviceContextGles3Inner {
    pub(crate) device_info: RafxDeviceInfo,

    gl_context_manager: GlContextManager,
    gl_context: Arc<GlContext>,
    destroyed: AtomicBool,
    pub(crate) validate_shaders: bool,

    pub(crate) fullscreen_quad: FullscreenQuad,
    pub(crate) gl_finish_call_count: AtomicU64,

    #[cfg(debug_assertions)]
    #[cfg(feature = "track-device-contexts")]
    next_create_index: AtomicU64,

    #[cfg(debug_assertions)]
    #[cfg(feature = "track-device-contexts")]
    pub(crate) all_contexts: Mutex<fnv::FnvHashMap<u64, backtrace::Backtrace>>,
}

// For GlContext
unsafe impl Send for RafxDeviceContextGles3Inner {}
unsafe impl Sync for RafxDeviceContextGles3Inner {}

impl Drop for RafxDeviceContextGles3Inner {
    fn drop(&mut self) {
        self.fullscreen_quad.destroy(&self.gl_context).unwrap();
        log::trace!("destroying device");
        self.destroyed.swap(true, Ordering::AcqRel);
    }
}

impl RafxDeviceContextGles3Inner {
    pub fn new(
        window: &dyn HasRawWindowHandle,
        gl_api_def: &RafxApiDefGles3,
    ) -> RafxResult<Self> {
        log::debug!("Initializing GL backend");
        let gl_context_manager = super::internal::GlContextManager::new(window)?;
        // GL requires a window for initialization
        let gl_context = gl_context_manager.main_context().clone();

        let renderer = gl_context.gl_get_string(gles3_bindings::RENDERER);
        log::debug!("Renderer: {}", renderer);
        let version = gl_context.gl_get_string(gles3_bindings::VERSION);
        log::debug!("Version: {}", version);
        let vendor = gl_context.gl_get_string(gles3_bindings::VENDOR);
        log::debug!("Vendor: {}", vendor);
        let shading_language_version =
            gl_context.gl_get_string(gles3_bindings::SHADING_LANGUAGE_VERSION);
        log::debug!("Shading Language Version: {}", shading_language_version);

        let pack_alignment = gl_context.gl_get_integerv(gles3_bindings::PACK_ALIGNMENT) as u32;
        let max_vertex_attribute_count =
            gl_context.gl_get_integerv(gles3_bindings::MAX_VERTEX_ATTRIBS) as u32;

        let min_uniform_buffer_offset_alignment =
            gl_context.gl_get_integerv(gles3_bindings::UNIFORM_BUFFER_OFFSET_ALIGNMENT) as u32;
        //let min_storage_buffer_offset_alignment = gl_context.gl_get_integerv(gles2_bindings::STORAGE_BUFFER_OFFSET_ALIGNMENT);

        let device_info = RafxDeviceInfo {
            supports_multithreaded_usage: false,
            min_uniform_buffer_offset_alignment,
            min_storage_buffer_offset_alignment: pack_alignment,
            upload_buffer_texture_alignment: pack_alignment,
            upload_buffer_texture_row_alignment: pack_alignment,
            supports_clamp_to_border_color: false, // requires GLES 3.2 or an extension
            max_vertex_attribute_count,
        };

        // Enable sRGB framebuffers on desktop GL. This is enabled by default on ES 3.0
        if gl_context.has_extension(&"GL_ARB_framebuffer_sRGB".to_string()) {
            // constant does not exist in bindings because they are based on ES 3.0 and
            // this is desktop-only
            const FRAMEBUFFER_SRGB: u32 = 0x8DB9;
            gl_context.gl_enable(FRAMEBUFFER_SRGB)?;
        }

        let fullscreen_quad = FullscreenQuad::new(&gl_context)?;

        #[cfg(debug_assertions)]
        #[cfg(feature = "track-device-contexts")]
        let all_contexts = {
            let create_backtrace = backtrace::Backtrace::new_unresolved();
            let mut all_contexts = fnv::FnvHashMap::<u64, backtrace::Backtrace>::default();
            all_contexts.insert(0, create_backtrace);
            all_contexts
        };

        Ok(RafxDeviceContextGles3Inner {
            device_info,
            gl_context_manager,
            gl_context,
            fullscreen_quad,
            destroyed: AtomicBool::new(false),
            validate_shaders: gl_api_def.validate_shaders,
            gl_finish_call_count: AtomicU64::new(0),

            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            all_contexts: Mutex::new(all_contexts),

            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            next_create_index: AtomicU64::new(1),
        })
    }
}

pub struct RafxDeviceContextGles3 {
    pub(crate) inner: Arc<RafxDeviceContextGles3Inner>,
    #[cfg(debug_assertions)]
    #[cfg(feature = "track-device-contexts")]
    pub(crate) create_index: u64,
}

impl std::fmt::Debug for RafxDeviceContextGles3 {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        f.debug_struct("RafxDeviceContextGl")
            //.field("handle", &self.device().handle())
            .finish()
    }
}

impl Clone for RafxDeviceContextGles3 {
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

            log::trace!("Cloned RafxDeviceContextGl create_index {}", create_index);
            create_index
        };

        RafxDeviceContextGles3 {
            inner: self.inner.clone(),
            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            create_index,
        }
    }
}

impl Drop for RafxDeviceContextGles3 {
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

impl Into<RafxDeviceContext> for RafxDeviceContextGles3 {
    fn into(self) -> RafxDeviceContext {
        RafxDeviceContext::Gles3(self)
    }
}

impl RafxDeviceContextGles3 {
    pub fn device_info(&self) -> &RafxDeviceInfo {
        &self.inner.device_info
    }

    pub fn gl_context(&self) -> &GlContext {
        &self.inner.gl_context
    }

    pub fn gl_context_manager(&self) -> &GlContextManager {
        &self.inner.gl_context_manager
    }

    // Used internally to support polling fences
    pub fn gl_finish(&self) -> RafxResult<()> {
        self.gl_context().gl_finish()?;
        self.inner
            .gl_finish_call_count
            .fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub fn new(inner: Arc<RafxDeviceContextGles3Inner>) -> RafxResult<Self> {
        Ok(RafxDeviceContextGles3 {
            inner,
            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            create_index: 0,
        })
    }

    pub fn create_queue(
        &self,
        queue_type: RafxQueueType,
    ) -> RafxResult<RafxQueueGles3> {
        RafxQueueGles3::new(self, queue_type)
    }

    pub fn create_fence(&self) -> RafxResult<RafxFenceGles3> {
        RafxFenceGles3::new(self)
    }

    pub fn create_semaphore(&self) -> RafxResult<RafxSemaphoreGles3> {
        RafxSemaphoreGles3::new(self)
    }

    pub fn create_swapchain(
        &self,
        raw_window_handle: &dyn HasRawWindowHandle,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<RafxSwapchainGles3> {
        RafxSwapchainGles3::new(self, raw_window_handle, swapchain_def)
    }

    pub fn wait_for_fences(
        &self,
        fences: &[&RafxFenceGles3],
    ) -> RafxResult<()> {
        RafxFenceGles3::wait_for_fences(self, fences)
    }

    pub fn create_sampler(
        &self,
        sampler_def: &RafxSamplerDef,
    ) -> RafxResult<RafxSamplerGles3> {
        RafxSamplerGles3::new(self, sampler_def)
    }

    pub fn create_texture(
        &self,
        texture_def: &RafxTextureDef,
    ) -> RafxResult<RafxTextureGles3> {
        RafxTextureGles3::new(self, texture_def)
    }

    pub fn create_buffer(
        &self,
        buffer_def: &RafxBufferDef,
    ) -> RafxResult<RafxBufferGles3> {
        RafxBufferGles3::new(self, buffer_def)
    }

    pub fn create_shader(
        &self,
        stages: Vec<RafxShaderStageDef>,
    ) -> RafxResult<RafxShaderGles3> {
        RafxShaderGles3::new(self, stages)
    }

    pub fn create_root_signature(
        &self,
        root_signature_def: &RafxRootSignatureDef,
    ) -> RafxResult<RafxRootSignatureGles3> {
        RafxRootSignatureGles3::new(self, root_signature_def)
    }

    pub fn create_descriptor_set_array(
        &self,
        descriptor_set_array_def: &RafxDescriptorSetArrayDef,
    ) -> RafxResult<RafxDescriptorSetArrayGles3> {
        RafxDescriptorSetArrayGles3::new(self, descriptor_set_array_def)
    }

    pub fn create_graphics_pipeline(
        &self,
        graphics_pipeline_def: &RafxGraphicsPipelineDef,
    ) -> RafxResult<RafxPipelineGles3> {
        RafxPipelineGles3::new_graphics_pipeline(self, graphics_pipeline_def)
    }

    pub fn create_compute_pipeline(
        &self,
        compute_pipeline_def: &RafxComputePipelineDef,
    ) -> RafxResult<RafxPipelineGles3> {
        RafxPipelineGles3::new_compute_pipeline(self, compute_pipeline_def)
    }

    pub fn create_shader_module(
        &self,
        data: RafxShaderModuleDefGles3,
    ) -> RafxResult<RafxShaderModuleGles3> {
        RafxShaderModuleGles3::new(self, data)
    }

    pub fn find_supported_format(
        &self,
        candidates: &[RafxFormat],
        resource_type: RafxResourceType,
    ) -> Option<RafxFormat> {
        if resource_type.intersects(RafxResourceType::RENDER_TARGET_DEPTH_STENCIL)
            || resource_type.intersects(RafxResourceType::RENDER_TARGET_COLOR)
        {
            for &candidate in candidates {
                if candidate.gles3_texture_format_info().is_some() {
                    return Some(candidate);
                }
            }

            return None;
        }

        None
    }

    pub fn find_supported_sample_count(
        &self,
        candidates: &[RafxSampleCount],
    ) -> Option<RafxSampleCount> {
        if candidates.contains(&RafxSampleCount::SampleCount1) {
            Some(RafxSampleCount::SampleCount1)
        } else {
            None
        }
    }
}
