use crate::gles3::{gles3_bindings, RafxDeviceContextGles3, ShaderId};
use crate::{RafxResult, RafxShaderModule, RafxShaderModuleDefGles3, RafxShaderStageFlags};
use rafx_base::trust_cell::TrustCell;
use std::ffi::{CStr, CString};
use std::sync::Arc;

#[derive(Debug)]
struct Gles3CompiledShaderInner {
    device_context: RafxDeviceContextGles3,
    shader_id: ShaderId,
    stage: RafxShaderStageFlags,
}

impl Drop for Gles3CompiledShaderInner {
    fn drop(&mut self) {
        self.device_context
            .gl_context()
            .gl_destroy_shader(self.shader_id)
            .unwrap();
    }
}

#[derive(Clone, Debug)]
pub struct Gles3CompiledShader {
    inner: Arc<Gles3CompiledShaderInner>,
}

impl Gles3CompiledShader {
    pub fn stage(&self) -> RafxShaderStageFlags {
        self.inner.stage
    }

    pub fn shader_id(&self) -> ShaderId {
        self.inner.shader_id
    }
}

pub struct RafxShaderModuleGles3Inner {
    device_context: RafxDeviceContextGles3,
    src: CString,
    compiled_shader: TrustCell<Option<Gles3CompiledShader>>,
}

impl std::fmt::Debug for RafxShaderModuleGles3Inner {
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
pub struct RafxShaderModuleGles3 {
    inner: Arc<RafxShaderModuleGles3Inner>,
}

impl RafxShaderModuleGles3 {
    pub fn src(&self) -> &CStr {
        &self.inner.src
    }

    pub fn new(
        device_context: &RafxDeviceContextGles3,
        data: RafxShaderModuleDefGles3,
    ) -> RafxResult<Self> {
        match data {
            RafxShaderModuleDefGles3::GlSrc(src) => {
                RafxShaderModuleGles3::new_from_src(device_context, src)
            }
        }
    }

    pub fn new_from_src(
        device_context: &RafxDeviceContextGles3,
        src: &str,
    ) -> RafxResult<Self> {
        let inner = RafxShaderModuleGles3Inner {
            device_context: device_context.clone(),
            compiled_shader: TrustCell::new(None),
            src: CString::new(src).map_err(|_| "Could not conver GL src from string to cstring")?,
        };

        Ok(RafxShaderModuleGles3 {
            inner: Arc::new(inner),
        })
    }

    pub(crate) fn compile_shader(
        &self,
        stage: RafxShaderStageFlags,
    ) -> RafxResult<Gles3CompiledShader> {
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
            gles3_bindings::VERTEX_SHADER
        } else if stage == RafxShaderStageFlags::FRAGMENT {
            gles3_bindings::FRAGMENT_SHADER
        } else {
            return Err(format!("Could not compile shader, stage flags must be EITHER vertex or fragment. Flags: {:?}", stage))?;
        };

        let gl_context = self.inner.device_context.gl_context();
        let shader_id = gl_context.compile_shader(gl_stage, &self.inner.src)?;

        let inner = Gles3CompiledShaderInner {
            device_context: self.inner.device_context.clone(),
            shader_id,
            stage,
        };

        let compiled_shader = Gles3CompiledShader {
            inner: Arc::new(inner),
        };

        *previously_compiled_shader = Some(compiled_shader.clone());

        Ok(compiled_shader)
    }
}

impl Into<RafxShaderModule> for RafxShaderModuleGles3 {
    fn into(self) -> RafxShaderModule {
        RafxShaderModule::Gles3(self)
    }
}
