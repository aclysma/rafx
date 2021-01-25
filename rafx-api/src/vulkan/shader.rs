use crate::vulkan::RafxDeviceContextVulkan;
use crate::{RafxPipelineReflection, RafxResult, RafxShaderStageDef, RafxShaderStageFlags};
use std::sync::Arc;

#[derive(Debug)]
struct RafxShaderVulkanInner {
    stage_flags: RafxShaderStageFlags,
    stages: Vec<RafxShaderStageDef>,
    pipeline_reflection: RafxPipelineReflection,
}

#[derive(Clone, Debug)]
pub struct RafxShaderVulkan {
    inner: Arc<RafxShaderVulkanInner>,
}

impl RafxShaderVulkan {
    pub fn new(
        _device_context: &RafxDeviceContextVulkan,
        stages: Vec<RafxShaderStageDef>,
    ) -> RafxResult<Self> {
        let pipeline_reflection = RafxPipelineReflection::from_stages(&stages)?;
        let mut stage_flags = RafxShaderStageFlags::empty();
        for stage in &stages {
            stage_flags |= stage.reflection.shader_stage;
        }

        let inner = RafxShaderVulkanInner {
            stages,
            pipeline_reflection,
            stage_flags,
        };

        Ok(RafxShaderVulkan {
            inner: Arc::new(inner),
        })
    }

    pub fn stages(&self) -> &[RafxShaderStageDef] {
        &self.inner.stages
    }

    pub fn pipeline_reflection(&self) -> &RafxPipelineReflection {
        &self.inner.pipeline_reflection
    }

    pub fn stage_flags(&self) -> RafxShaderStageFlags {
        self.inner.stage_flags
    }
}
