use crate::gl::RafxDeviceContextGl;
use crate::{RafxPipelineReflection, RafxResult, RafxShaderStageDef, RafxShaderStageFlags};
use std::sync::Arc;

#[derive(Debug)]
struct RafxShaderGlInner {
    stage_flags: RafxShaderStageFlags,
    stages: Vec<RafxShaderStageDef>,
    pipeline_reflection: RafxPipelineReflection,
}

#[derive(Clone, Debug)]
pub struct RafxShaderGl {
    inner: Arc<RafxShaderGlInner>,
}

impl RafxShaderGl {
    pub fn new(
        _device_context: &RafxDeviceContextGl,
        stages: Vec<RafxShaderStageDef>,
    ) -> RafxResult<Self> {
        unimplemented!();
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
