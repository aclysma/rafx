use crate::gl::{RafxDeviceContextGl, gles20, ShaderId};
use crate::{RafxResult, RafxShaderModule, RafxShaderModuleDefGl, RafxShaderStageFlags};
use std::sync::{Arc, Mutex};
use std::ffi::{CString, CStr};
use rafx_base::trust_cell::TrustCell;

struct CompiledShader {
    shader_id: ShaderId,
    stage: RafxShaderStageFlags,
}

pub struct RafxShaderModuleGlInner {
    device_context: RafxDeviceContextGl,
    src: CString,
    compiled_shader: TrustCell<Option<CompiledShader>>,
}

impl std::fmt::Debug for RafxShaderModuleGlInner {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("RafxShaderModuleGlInner")
            .field("device_context", &self.device_context)
            .finish()
    }
}

// for gl_rs::Library
unsafe impl Send for RafxShaderModuleGlInner {}
unsafe impl Sync for RafxShaderModuleGlInner {}

#[derive(Clone, Debug)]
pub struct RafxShaderModuleGl {
    inner: Arc<RafxShaderModuleGlInner>,
}

impl RafxShaderModuleGl {
    pub fn src(&self) -> &CStr {
        &self.inner.src
    }

    pub fn new(
        device_context: &RafxDeviceContextGl,
        data: RafxShaderModuleDefGl,
    ) -> RafxResult<Self> {
        match data {
            RafxShaderModuleDefGl::GlSrc(src) => {
                RafxShaderModuleGl::new_from_src(device_context, src)
            }
        }
    }

    pub fn new_from_src(
        device_context: &RafxDeviceContextGl,
        src: &str,
    ) -> RafxResult<Self> {
        let inner = RafxShaderModuleGlInner {
            device_context: device_context.clone(),
            compiled_shader: TrustCell::new(None),
            src: CString::new(src).map_err(|_| "Could not conver GL src from string to cstring")?
        };

        Ok(RafxShaderModuleGl {
            inner: Arc::new(inner),
        })
    }

    pub(crate) fn compile_shader(&self, stage: RafxShaderStageFlags) -> RafxResult<ShaderId> {
        let mut compiled_shader = self.inner.compiled_shader.borrow_mut();
        if let Some(compiled_shader) = compiled_shader.as_ref() {
            return if compiled_shader.stage == stage {
                log::debug!("compile_shader called, returning previously compiled result");
                Ok(compiled_shader.shader_id)
            } else {
                Err(format!("Shader was already compiled with stage {:?}, but compile_shader() called again with stage {:?}", compiled_shader.stage, stage))?
            };
        }

        let gl_stage = if stage == RafxShaderStageFlags::VERTEX {
            gles20::VERTEX_SHADER
        } else if stage == RafxShaderStageFlags::FRAGMENT {
            gles20::FRAGMENT_SHADER
        } else {
            return Err(format!("Could not compile shader, stage flags must be EITHER vertex or fragment. Flags: {:?}", stage))?;
        };

        let gl_context = self.inner.device_context.gl_context();
        let shader_id = gl_context.compile_shader(gl_stage, &self.inner.src)?;

        *compiled_shader = Some(CompiledShader {
            shader_id,
            stage
        });

        Ok(shader_id)
    }
}

impl Into<RafxShaderModule> for RafxShaderModuleGl {
    fn into(self) -> RafxShaderModule {
        RafxShaderModule::Gl(self)
    }
}
