use crate::gl::RafxDeviceContextGl;
use crate::{RafxResult, RafxShaderModule, RafxShaderModuleDefGl};
use std::sync::Arc;

#[derive(Debug)]
pub struct RafxShaderModuleGlInner {
    //library: gl_rs::Library,
}

// for gl_rs::Library
unsafe impl Send for RafxShaderModuleGlInner {}
unsafe impl Sync for RafxShaderModuleGlInner {}

#[derive(Clone, Debug)]
pub struct RafxShaderModuleGl {
    inner: Arc<RafxShaderModuleGlInner>,
}

impl RafxShaderModuleGl {
    // pub fn library(&self) -> &gl_rs::LibraryRef {
    //     self.inner.library.as_ref()
    // }

    pub fn new(
        device_context: &RafxDeviceContextGl,
        data: RafxShaderModuleDefGl,
    ) -> RafxResult<Self> {
        unimplemented!();
        // match data {
        //     RafxShaderModuleDefGl::GlLibBytes(bytes) => {
        //         RafxShaderModuleGl::new_from_lib_bytes(device_context, bytes)
        //     }
        //     RafxShaderModuleDefGl::GlSrc(spv) => {
        //         RafxShaderModuleGl::new_from_src(device_context, spv)
        //     }
        // }
    }

    pub fn new_from_lib_bytes(
        device_context: &RafxDeviceContextGl,
        data: &[u8],
    ) -> RafxResult<Self> {
        unimplemented!();
        // let library = device_context.device().new_library_with_data(data)?;
        //
        // let inner = RafxShaderModuleGlInner { library };
        //
        // Ok(RafxShaderModuleGl {
        //     inner: Arc::new(inner),
        // })
    }

    pub fn new_from_src(
        device_context: &RafxDeviceContextGl,
        src: &str,
    ) -> RafxResult<Self> {
        unimplemented!();
        // let compile_options = gl_rs::CompileOptions::new();
        // compile_options.set_language_version(MTLLanguageVersion::V2_1);
        // let library = device_context
        //     .device()
        //     .new_library_with_source(src, &compile_options)?;
        //
        // let inner = RafxShaderModuleGlInner { library };
        //
        // Ok(RafxShaderModuleGl {
        //     inner: Arc::new(inner),
        // })
    }
}

impl Into<RafxShaderModule> for RafxShaderModuleGl {
    fn into(self) -> RafxShaderModule {
        RafxShaderModule::Gl(self)
    }
}
