use crate::{RafxApiDef, RafxResult, RafxValidationMode};
use raw_window_handle::HasRawWindowHandle;
use std::sync::Arc;

use crate::dx12::{RafxDeviceContextDx12, RafxDeviceContextDx12Inner};

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq)]
pub enum RafxDx12FeatureLevel {
    Dx12FeatureLevel_12_0,
    Dx12FeatureLevel_12_1,
    Dx12FeatureLevel_12_2,
}

impl Default for RafxDx12FeatureLevel {
    fn default() -> Self {
        RafxDx12FeatureLevel::Dx12FeatureLevel_12_0
    }
}

/// Dx12-specific configuration
#[derive(Default)]
pub struct RafxApiDefDx12 {
    /// Used to enable/disable validation at runtime. Not all APIs allow this. Validation is helpful
    /// during development but very expensive. Applications should not ship with validation enabled.
    pub validation_mode: RafxValidationMode,

    /// Enable the tagging of dx12 objects with debug names.
    pub enable_debug_names: bool,

    /// Use windows WARP (software rendering)
    pub use_warp_device: bool,

    pub minimum_feature_level: RafxDx12FeatureLevel,

    pub enable_gpu_based_validation: bool,
}

pub struct RafxApiDx12 {
    device_context: Option<RafxDeviceContextDx12>,
}

impl Drop for RafxApiDx12 {
    fn drop(&mut self) {
        self.destroy().unwrap();
    }
}

impl RafxApiDx12 {
    pub fn device_context(&self) -> &RafxDeviceContextDx12 {
        self.device_context.as_ref().unwrap()
    }

    /// # Safety
    ///
    /// GPU programming is fundamentally unsafe, so all rafx APIs that interact with the GPU should
    /// be considered unsafe. However, rafx APIs are only gated by unsafe if they can cause undefined
    /// behavior on the CPU for reasons other than interacting with the GPU.
    pub unsafe fn new(
        _window: &dyn HasRawWindowHandle,
        _api_def: &RafxApiDef,
        dx12_api_def: &RafxApiDefDx12,
    ) -> RafxResult<Self> {
        let inner = Arc::new(RafxDeviceContextDx12Inner::new(dx12_api_def)?);
        let device_context = RafxDeviceContextDx12::new(inner)?;

        Ok(RafxApiDx12 {
            device_context: Some(device_context),
        })
    }

    pub fn destroy(&mut self) -> RafxResult<()> {
        if let Some(device_context) = self.device_context.take() {
            // Clear any internal caches that may hold references to the device
            let inner = device_context.inner.clone();
            //inner.descriptor_heap.clear_pools(device_context.device());
            //inner.resource_cache.clear_caches();

            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            let _create_index = device_context.create_index;

            // This should be the final device context
            std::mem::drop(device_context);

            let _strong_count = Arc::strong_count(&inner);
            match Arc::try_unwrap(inner) {
                Ok(inner) => std::mem::drop(inner),
                Err(_arc) => {
                    Err(format!(
                        "Could not destroy device, {} references to it exist",
                        _strong_count
                    ))?;

                    #[cfg(debug_assertions)]
                    #[cfg(feature = "track-device-contexts")]
                    {
                        let mut all_contexts = _arc.all_contexts.lock().unwrap();
                        all_contexts.remove(&_create_index);
                        for (k, v) in all_contexts.iter_mut() {
                            v.resolve();
                            println!("context allocation: {}\n{:?}", k, v);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
