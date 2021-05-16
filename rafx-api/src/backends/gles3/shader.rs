use crate::gles3::{Gles3CompiledShader, ProgramId, RafxDeviceContextGles3};
use crate::{RafxPipelineReflection, RafxResult, RafxShaderStageDef, RafxShaderStageFlags};
use std::sync::Arc;

#[derive(Debug)]
struct RafxShaderGles3Inner {
    device_context: RafxDeviceContextGles3,
    stage_flags: RafxShaderStageFlags,
    stages: Vec<RafxShaderStageDef>,
    pipeline_reflection: RafxPipelineReflection,
    vertex_shader: Gles3CompiledShader,
    fragment_shader: Gles3CompiledShader,
    program_id: ProgramId,
}

impl Drop for RafxShaderGles3Inner {
    fn drop(&mut self) {
        self.device_context
            .gl_context()
            .gl_destroy_program(self.program_id)
            .unwrap();
    }
}

#[derive(Clone, Debug)]
pub struct RafxShaderGles3 {
    inner: Arc<RafxShaderGles3Inner>,
}

impl RafxShaderGles3 {
    pub fn stages(&self) -> &[RafxShaderStageDef] {
        &self.inner.stages
    }

    pub fn pipeline_reflection(&self) -> &RafxPipelineReflection {
        &self.inner.pipeline_reflection
    }

    pub fn stage_flags(&self) -> RafxShaderStageFlags {
        self.inner.stage_flags
    }

    pub fn gl_program_id(&self) -> ProgramId {
        self.inner.program_id
    }

    pub fn gl_vertex_shader(&self) -> &Gles3CompiledShader {
        &self.inner.vertex_shader
    }

    pub fn gl_fragment_shader(&self) -> &Gles3CompiledShader {
        &self.inner.fragment_shader
    }

    pub fn new(
        device_context: &RafxDeviceContextGles3,
        stages: Vec<RafxShaderStageDef>,
    ) -> RafxResult<Self> {
        let pipeline_reflection = RafxPipelineReflection::from_stages(&stages)?;
        let mut stage_flags = RafxShaderStageFlags::empty();

        let mut vertex_shader_id = None;
        let mut fragment_shader_id = None;

        for stage in &stages {
            stage_flags |= stage.reflection.shader_stage;

            log::debug!(
                "Compiling shader for stage {:?}",
                stage.reflection.shader_stage
            );
            let compiled = stage
                .shader_module
                .gles3_shader_module()
                .unwrap()
                .compile_shader(stage.reflection.shader_stage)?;
            if stage.reflection.shader_stage == RafxShaderStageFlags::VERTEX {
                vertex_shader_id = Some(compiled);
            } else if stage.reflection.shader_stage == RafxShaderStageFlags::FRAGMENT {
                fragment_shader_id = Some(compiled);
            } else {
                return Err(format!(
                    "Unexpected shader stage for GL ES 2.0: {:?}",
                    stage.reflection.shader_stage
                ))?;
            }
        }

        let vertex_shader =
            vertex_shader_id.ok_or("No vertex shader specified, it is required for GL ES 2.0")?;
        let fragment_shader = fragment_shader_id
            .ok_or("No fragment shader specified, it is required for GL ES 2.0")?;

        let gl_context = device_context.gl_context();
        let program_id = gl_context.gl_create_program()?;
        gl_context.gl_attach_shader(program_id, vertex_shader.shader_id())?;
        gl_context.gl_attach_shader(program_id, fragment_shader.shader_id())?;

        gl_context.link_shader_program(program_id)?;

        let inner = RafxShaderGles3Inner {
            device_context: device_context.clone(),
            stages,
            pipeline_reflection,
            stage_flags,
            vertex_shader,
            fragment_shader,
            program_id,
        };

        Ok(RafxShaderGles3 {
            inner: Arc::new(inner),
        })
    }
}
