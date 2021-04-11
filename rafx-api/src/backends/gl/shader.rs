use crate::gl::{RafxDeviceContextGl, ShaderId};
use crate::{RafxPipelineReflection, RafxResult, RafxShaderStageDef, RafxShaderStageFlags};
use std::sync::Arc;

#[derive(Debug)]
struct RafxShaderGlInner {
    stage_flags: RafxShaderStageFlags,
    stages: Vec<RafxShaderStageDef>,
    pipeline_reflection: RafxPipelineReflection,
    vertex_shader: ShaderId,
    fragment_shader: ShaderId,
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

        let mut vertex_shader = None;
        let mut fragment_shader = None;

        for stage in &stages {
            stage_flags |= stage.reflection.shader_stage;

            let compiled = stage.shader_module.gl_shader_module().unwrap().compile_shader(stage.reflection.shader_stage)?;
            if stage == RafxShaderStageFlags::VERTEX {
                vertex_shader = Some(compiled);
            } else if stage == RafxShaderStageFlags::FRAGMENT {
                fragment_shader = Some(compiled);
            } else {
                return Err(format!("Unexpected shader stage for GL ES 2.0: {:?}", stage.reflection.shader_stage))?;
            }
        }

        let vertex_shader = vertex_shader.ok_or(Err("No vertex shader specified, it is required for GL ES 2.0"))?;
        let fragment_shader = fragment_shader.ok_or(Err("No fragment shader specified, it is required for GL ES 2.0"))?;

        let gl_context = device_context.gl_context();
        let program = gl_context.gl_create_program()?;
        gl_context.gl_attach_shader(program, vertex_shader)?;
        gl_context.gl_attach_shader(program, fragment_shader)?;

        gl_context.link_shader(program);



        let inner = RafxShaderGlInner {
            stages,
            pipeline_reflection,
            stage_flags,
        };

        Ok(RafxShaderGl {
            inner: Arc::new(inner),
        })

        //unimplemented!();
        // let pipeline_reflection = RafxPipelineReflection::from_stages(&stages)?;
        // let mut stage_flags = RafxShaderStageFlags::empty();
        // for stage in &stages {
        //     stage_flags |= stage.reflection.shader_stage;
        // }
        //
        // let inner = RafxShaderGlInner {
        //     stages,
        //     pipeline_reflection,
        //     stage_flags,
        // };
        //
        // Ok(RafxShaderGl {
        //     inner: Arc::new(inner),
        // })
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
