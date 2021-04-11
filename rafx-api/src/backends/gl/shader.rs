use crate::gl::{RafxDeviceContextGl, ShaderId, ProgramId};
use crate::{RafxPipelineReflection, RafxResult, RafxShaderStageDef, RafxShaderStageFlags};
use std::sync::Arc;

#[derive(Debug)]
struct RafxShaderGlInner {
    stage_flags: RafxShaderStageFlags,
    stages: Vec<RafxShaderStageDef>,
    pipeline_reflection: RafxPipelineReflection,
    vertex_shader_id: ShaderId,
    fragment_shader_id: ShaderId,
    program_id: ProgramId,
}

#[derive(Clone, Debug)]
pub struct RafxShaderGl {
    inner: Arc<RafxShaderGlInner>,
}

impl RafxShaderGl {
    pub fn new(
        device_context: &RafxDeviceContextGl,
        stages: Vec<RafxShaderStageDef>,
    ) -> RafxResult<Self> {
        let pipeline_reflection = RafxPipelineReflection::from_stages(&stages)?;
        let mut stage_flags = RafxShaderStageFlags::empty();

        let mut vertex_shader_id = None;
        let mut fragment_shader_id = None;

        for stage in &stages {
            stage_flags |= stage.reflection.shader_stage;

            log::debug!("Compiling shader for stage {:?}", stage.reflection.shader_stage);
            let compiled = stage.shader_module.gl_shader_module().unwrap().compile_shader(stage.reflection.shader_stage)?;
            if stage.reflection.shader_stage == RafxShaderStageFlags::VERTEX {
                vertex_shader_id = Some(compiled);
            } else if stage.reflection.shader_stage == RafxShaderStageFlags::FRAGMENT {
                fragment_shader_id = Some(compiled);
            } else {
                return Err(format!("Unexpected shader stage for GL ES 2.0: {:?}", stage.reflection.shader_stage))?;
            }
        }

        let vertex_shader_id = vertex_shader_id.ok_or("No vertex shader specified, it is required for GL ES 2.0")?;
        let fragment_shader_id = fragment_shader_id.ok_or("No fragment shader specified, it is required for GL ES 2.0")?;

        let gl_context = device_context.gl_context();
        let program_id = gl_context.gl_create_program()?;
        gl_context.gl_attach_shader(program_id, vertex_shader_id)?;
        gl_context.gl_attach_shader(program_id, fragment_shader_id)?;

        gl_context.link_and_validate_shader_program(program_id)?;

        let inner = RafxShaderGlInner {
            stages,
            pipeline_reflection,
            stage_flags,
            vertex_shader_id,
            fragment_shader_id,
            program_id
        };

        Ok(RafxShaderGl {
            inner: Arc::new(inner),
        })
    }

    pub fn stages(&self) -> &[RafxShaderStageDef] {
        unimplemented!();
        &self.inner.stages
    }

    pub fn pipeline_reflection(&self) -> &RafxPipelineReflection {
        unimplemented!();
        &self.inner.pipeline_reflection
    }

    pub fn stage_flags(&self) -> RafxShaderStageFlags {
        unimplemented!();
        self.inner.stage_flags
    }
}
