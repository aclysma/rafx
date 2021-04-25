use crate::gl::{gles20, RafxDeviceContextGles2, ShaderId};
use crate::{RafxResult, RafxShaderModule, RafxShaderModuleDefGles2, RafxShaderStageFlags};
use rafx_base::trust_cell::TrustCell;
use std::ffi::{CStr, CString};
use std::sync::Arc;

#[derive(Debug)]
struct Gles2CompiledShaderInner {
    device_context: RafxDeviceContextGles2,
    shader_id: ShaderId,
    stage: RafxShaderStageFlags,
}

impl Drop for Gles2CompiledShaderInner {
    fn drop(&mut self) {
        self.device_context
            .gl_context()
            .gl_destroy_shader(self.shader_id)
            .unwrap();
    }
}

#[derive(Clone, Debug)]
pub struct Gles2CompiledShader {
    inner: Arc<Gles2CompiledShaderInner>,
}

impl Gles2CompiledShader {
    pub fn stage(&self) -> RafxShaderStageFlags {
        self.inner.stage
    }

    pub fn shader_id(&self) -> ShaderId {
        self.inner.shader_id
    }
}

pub struct RafxShaderModuleGles2Inner {
    device_context: RafxDeviceContextGles2,
    src: CString,
    compiled_shader: TrustCell<Option<Gles2CompiledShader>>,
}

impl std::fmt::Debug for RafxShaderModuleGles2Inner {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("RafxShaderModuleGlInner")
            .field("device_context", &self.device_context)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct RafxShaderModuleGles2 {
    inner: Arc<RafxShaderModuleGles2Inner>,
}

impl RafxShaderModuleGles2 {
    pub fn src(&self) -> &CStr {
        &self.inner.src
    }

    pub fn new(
        device_context: &RafxDeviceContextGles2,
        data: RafxShaderModuleDefGles2,
    ) -> RafxResult<Self> {
        match data {
            RafxShaderModuleDefGles2::GlSrc(src) => {
                RafxShaderModuleGles2::new_from_src(device_context, src)
            }
        }
    }

    pub fn new_from_src(
        device_context: &RafxDeviceContextGles2,
        src: &str,
    ) -> RafxResult<Self> {
        let inner = RafxShaderModuleGles2Inner {
            device_context: device_context.clone(),
            compiled_shader: TrustCell::new(None),
            src: CString::new(src).map_err(|_| "Could not conver GL src from string to cstring")?,
        };

        Ok(RafxShaderModuleGles2 {
            inner: Arc::new(inner),
        })
    }

    pub(crate) fn compile_shader(
        &self,
        stage: RafxShaderStageFlags,
    ) -> RafxResult<Gles2CompiledShader> {
        let mut previously_compiled_shader = self.inner.compiled_shader.borrow_mut();
        if let Some(compiled_shader) = previously_compiled_shader.as_ref() {
            return if compiled_shader.stage() == stage {
                log::debug!("compile_shader called, returning previously compiled result");
                Ok(compiled_shader.clone())
            } else {
                Err(format!("Shader was already compiled with stage {:?}, but compile_shader() called again with stage {:?}", compiled_shader.stage(), stage))?
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

        let inner = Gles2CompiledShaderInner {
            device_context: self.inner.device_context.clone(),
            shader_id,
            stage,
        };

        let compiled_shader = Gles2CompiledShader {
            inner: Arc::new(inner),
        };

        *previously_compiled_shader = Some(compiled_shader.clone());

        Ok(compiled_shader)
    }
}

impl Into<RafxShaderModule> for RafxShaderModuleGles2 {
    fn into(self) -> RafxShaderModule {
        RafxShaderModule::Gles2(self)
    }
}
