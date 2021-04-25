use crate::{
    RafxBufferDef, RafxComputePipelineDef, RafxDescriptorSetArrayDef, RafxDeviceContext,
    RafxDeviceInfo, RafxFormat, RafxGraphicsPipelineDef, RafxQueueType, RafxResourceType,
    RafxResult, RafxRootSignatureDef, RafxSampleCount, RafxSamplerDef, RafxShaderModuleDefGl,
    RafxShaderStageDef, RafxSwapchainDef, RafxTextureDef,
};
use raw_window_handle::HasRawWindowHandle;
use std::sync::Arc;

use crate::gl::{
    GlContextManager, RafxBufferGles2, RafxDescriptorSetArrayGles2, RafxFenceGles2, RafxPipelineGles2,
    RafxQueueGles2, RafxRootSignatureGles2, RafxSamplerGles2, RafxSemaphoreGles2, RafxShaderGles2,
    RafxShaderModuleGles2, RafxSwapchainGles2, RafxTextureGles2,
};

use crate::gl::gles20;
use crate::gl::GlContext;

use crate::gl::fullscreen_quad::FullscreenQuad;

#[cfg(debug_assertions)]
#[cfg(feature = "track-device-contexts")]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct RafxDeviceContextGles2Inner {
    pub(crate) device_info: RafxDeviceInfo,

    gl_context_manager: GlContextManager,
    gl_context: Arc<GlContext>,
    destroyed: AtomicBool,

    pub(crate) fullscreen_quad: FullscreenQuad,

    #[cfg(debug_assertions)]
    #[cfg(feature = "track-device-contexts")]
    next_create_index: AtomicU64,

    #[cfg(debug_assertions)]
    #[cfg(feature = "track-device-contexts")]
    pub(crate) all_contexts: Mutex<fnv::FnvHashMap<u64, backtrace::Backtrace>>,
}

// For GlContext
unsafe impl Send for RafxDeviceContextGles2Inner {}
unsafe impl Sync for RafxDeviceContextGles2Inner {}

impl Drop for RafxDeviceContextGles2Inner {
    fn drop(&mut self) {
        self.fullscreen_quad.destroy(&self.gl_context).unwrap();
        log::trace!("destroying device");
        self.destroyed.swap(true, Ordering::AcqRel);
    }
}

impl RafxDeviceContextGles2Inner {
    pub fn new(window: &dyn HasRawWindowHandle) -> RafxResult<Self> {
        log::debug!("Initializing GL backend");
        let gl_context_manager = super::internal::GlContextManager::new(window)?;
        // GL requires a window for initialization
        let gl_context = gl_context_manager.main_context().clone();

        let renderer = gl_context.gl_get_string(gles20::RENDERER);
        log::debug!("Renderer: {}", renderer);
        let version = gl_context.gl_get_string(gles20::VERSION);
        log::debug!("Version: {}", version);
        let vendor = gl_context.gl_get_string(gles20::VENDOR);
        log::debug!("Vendor: {}", vendor);
        let shading_language_version = gl_context.gl_get_string(gles20::SHADING_LANGUAGE_VERSION);
        log::debug!("Shading Language Version: {}", shading_language_version);

        let pack_alignment = gl_context.gl_get_integerv(gles20::PACK_ALIGNMENT) as u32;
        let max_vertex_attribute_count =
            gl_context.gl_get_integerv(gles20::MAX_VERTEX_ATTRIBS) as u32;

        let device_info = RafxDeviceInfo {
            min_uniform_buffer_offset_alignment: pack_alignment,
            min_storage_buffer_offset_alignment: pack_alignment,
            upload_buffer_texture_alignment: pack_alignment,
            upload_buffer_texture_row_alignment: pack_alignment,
            supports_clamp_to_border_color: false, // requires GLES 3.2 or an extension
            max_vertex_attribute_count,
        };

        let fullscreen_quad = FullscreenQuad::new(&gl_context)?;

        //TODO: Support extensions

        #[cfg(debug_assertions)]
        #[cfg(feature = "track-device-contexts")]
        let all_contexts = {
            let create_backtrace = backtrace::Backtrace::new_unresolved();
            let mut all_contexts = fnv::FnvHashMap::<u64, backtrace::Backtrace>::default();
            all_contexts.insert(0, create_backtrace);
            all_contexts
        };

        Ok(RafxDeviceContextGles2Inner {
            device_info,
            gl_context_manager,
            gl_context,
            fullscreen_quad,
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

pub struct RafxDeviceContextGles2 {
    pub(crate) inner: Arc<RafxDeviceContextGles2Inner>,
    #[cfg(debug_assertions)]
    #[cfg(feature = "track-device-contexts")]
    pub(crate) create_index: u64,
}

impl std::fmt::Debug for RafxDeviceContextGles2 {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        f.debug_struct("RafxDeviceContextGl")
            //.field("handle", &self.device().handle())
            .finish()
    }
}

impl Clone for RafxDeviceContextGles2 {
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

        RafxDeviceContextGles2 {
            inner: self.inner.clone(),
            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            create_index,
        }
    }
}

impl Drop for RafxDeviceContextGles2 {
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

impl Into<RafxDeviceContext> for RafxDeviceContextGles2 {
    fn into(self) -> RafxDeviceContext {
        RafxDeviceContext::Gl(self)
    }
}

impl RafxDeviceContextGles2 {
    pub fn device_info(&self) -> &RafxDeviceInfo {
        &self.inner.device_info
    }

    // pub fn device(&self) -> &gl_rs::Device {
    //     &self.inner.device
    // }

    // pub fn gl_features(&self) -> &GlFeatures {
    //     &self.inner.gl_features
    // }

    pub fn gl_context(&self) -> &GlContext {
        &self.inner.gl_context
    }

    pub fn gl_context_manager(&self) -> &GlContextManager {
        &self.inner.gl_context_manager
    }

    pub fn new(inner: Arc<RafxDeviceContextGles2Inner>) -> RafxResult<Self> {
        Ok(RafxDeviceContextGles2 {
            inner,
            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            create_index: 0,
        })
    }

    pub fn create_queue(
        &self,
        queue_type: RafxQueueType,
    ) -> RafxResult<RafxQueueGles2> {
        RafxQueueGles2::new(self, queue_type)
    }

    pub fn create_fence(&self) -> RafxResult<RafxFenceGles2> {
        RafxFenceGles2::new(self)
    }

    pub fn create_semaphore(&self) -> RafxResult<RafxSemaphoreGles2> {
        RafxSemaphoreGles2::new(self)
    }

    pub fn create_swapchain(
        &self,
        raw_window_handle: &dyn HasRawWindowHandle,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<RafxSwapchainGles2> {
        RafxSwapchainGles2::new(self, raw_window_handle, swapchain_def)
    }

    pub fn wait_for_fences(
        &self,
        fences: &[&RafxFenceGles2],
    ) -> RafxResult<()> {
        RafxFenceGles2::wait_for_fences(self, fences)
    }

    pub fn create_sampler(
        &self,
        sampler_def: &RafxSamplerDef,
    ) -> RafxResult<RafxSamplerGles2> {
        RafxSamplerGles2::new(self, sampler_def)
    }

    pub fn create_texture(
        &self,
        texture_def: &RafxTextureDef,
    ) -> RafxResult<RafxTextureGles2> {
        RafxTextureGles2::new(self, texture_def)
    }

    pub fn create_buffer(
        &self,
        buffer_def: &RafxBufferDef,
    ) -> RafxResult<RafxBufferGles2> {
        RafxBufferGles2::new(self, buffer_def)
    }

    pub fn create_shader(
        &self,
        stages: Vec<RafxShaderStageDef>,
    ) -> RafxResult<RafxShaderGles2> {
        RafxShaderGles2::new(self, stages)
    }

    pub fn create_root_signature(
        &self,
        root_signature_def: &RafxRootSignatureDef,
    ) -> RafxResult<RafxRootSignatureGles2> {
        RafxRootSignatureGles2::new(self, root_signature_def)
    }

    pub fn create_descriptor_set_array(
        &self,
        descriptor_set_array_def: &RafxDescriptorSetArrayDef,
    ) -> RafxResult<RafxDescriptorSetArrayGles2> {
        RafxDescriptorSetArrayGles2::new(self, descriptor_set_array_def)
    }

    pub fn create_graphics_pipeline(
        &self,
        graphics_pipeline_def: &RafxGraphicsPipelineDef,
    ) -> RafxResult<RafxPipelineGles2> {
        RafxPipelineGles2::new_graphics_pipeline(self, graphics_pipeline_def)
    }

    pub fn create_compute_pipeline(
        &self,
        compute_pipeline_def: &RafxComputePipelineDef,
    ) -> RafxResult<RafxPipelineGles2> {
        RafxPipelineGles2::new_compute_pipeline(self, compute_pipeline_def)
    }

    pub fn create_shader_module(
        &self,
        data: RafxShaderModuleDefGl,
    ) -> RafxResult<RafxShaderModuleGles2> {
        RafxShaderModuleGles2::new(self, data)
    }

    pub fn find_supported_format(
        &self,
        candidates: &[RafxFormat],
        _resource_type: RafxResourceType,
    ) -> Option<RafxFormat> {
        // OpenGL doesn't provide a great way to determine if a texture is natively available
        for &candidate in candidates {
            // For now we don't support compressed textures for GL ES at all (but we probably could)
            if !candidate.is_compressed() {
                return Some(candidate);
            }
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
