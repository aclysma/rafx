use crate::metal::RafxDeviceContextMetal;
use crate::{RafxResult, RafxShaderModule, RafxShaderModuleDefMetal};
use metal_rs::MTLLanguageVersion;
use std::sync::Arc;

#[derive(Debug)]
pub struct RafxShaderModuleMetalInner {
    library: metal_rs::Library,
}

// for metal_rs::Library
unsafe impl Send for RafxShaderModuleMetalInner {}
unsafe impl Sync for RafxShaderModuleMetalInner {}

#[derive(Clone, Debug)]
pub struct RafxShaderModuleMetal {
    inner: Arc<RafxShaderModuleMetalInner>,
}

impl RafxShaderModuleMetal {
    pub fn library(&self) -> &metal_rs::LibraryRef {
        self.inner.library.as_ref()
    }

    pub fn new(
        device_context: &RafxDeviceContextMetal,
        data: RafxShaderModuleDefMetal,
    ) -> RafxResult<Self> {
        match data {
            RafxShaderModuleDefMetal::MetalLibBytes(bytes) => {
                RafxShaderModuleMetal::new_from_lib_bytes(device_context, bytes)
            }
            RafxShaderModuleDefMetal::MetalSrc(spv) => {
                RafxShaderModuleMetal::new_from_src(device_context, spv)
            }
        }
    }

    pub fn new_from_lib_bytes(
        device_context: &RafxDeviceContextMetal,
        data: &[u8],
    ) -> RafxResult<Self> {
        let library = device_context.device().new_library_with_data(data)?;

        let inner = RafxShaderModuleMetalInner { library };

        Ok(RafxShaderModuleMetal {
            inner: Arc::new(inner),
        })
    }

    pub fn new_from_src(
        device_context: &RafxDeviceContextMetal,
        src: &str,
    ) -> RafxResult<Self> {
        let compile_options = metal_rs::CompileOptions::new();
        compile_options.set_language_version(MTLLanguageVersion::V2_1);
        let library = device_context
            .device()
            .new_library_with_source(src, &compile_options)?;

        let inner = RafxShaderModuleMetalInner { library };

        Ok(RafxShaderModuleMetal {
            inner: Arc::new(inner),
        })
    }
}

impl Into<RafxShaderModule> for RafxShaderModuleMetal {
    fn into(self) -> RafxShaderModule {
        RafxShaderModule::Metal(self)
    }
}
