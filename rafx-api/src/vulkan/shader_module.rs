use crate::{RafxResult, RafxShaderModule, RafxShaderModuleDefVulkan};

use crate::vulkan::RafxDeviceContextVulkan;
use ash::version::DeviceV1_0;
use ash::vk;
use std::sync::Arc;

#[derive(Debug)]
pub struct RafxShaderModuleVulkanInner {
    device_context: RafxDeviceContextVulkan,
    shader_module: vk::ShaderModule,
}

impl Drop for RafxShaderModuleVulkanInner {
    fn drop(&mut self) {
        unsafe {
            self.device_context
                .device()
                .destroy_shader_module(self.shader_module, None)
        }
    }
}

#[derive(Clone, Debug)]
pub struct RafxShaderModuleVulkan {
    inner: Arc<RafxShaderModuleVulkanInner>,
}

impl RafxShaderModuleVulkan {
    pub fn new(
        device_context: &RafxDeviceContextVulkan,
        data: RafxShaderModuleDefVulkan,
    ) -> RafxResult<Self> {
        match data {
            RafxShaderModuleDefVulkan::VkSpvBytes(bytes) => {
                RafxShaderModuleVulkan::new_from_bytes(device_context, bytes)
            }
            RafxShaderModuleDefVulkan::VkSpvPrepared(spv) => {
                RafxShaderModuleVulkan::new_from_spv(device_context, spv)
            }
        }
    }

    pub fn new_from_bytes(
        device_context: &RafxDeviceContextVulkan,
        data: &[u8],
    ) -> RafxResult<Self> {
        let spv = ash::util::read_spv(&mut std::io::Cursor::new(data))?;
        Self::new_from_spv(device_context, &spv)
    }

    pub fn new_from_spv(
        device_context: &RafxDeviceContextVulkan,
        data: &[u32],
    ) -> RafxResult<Self> {
        let create_info = vk::ShaderModuleCreateInfo::builder().code(&data);

        let shader_module = unsafe {
            device_context
                .device()
                .create_shader_module(&create_info, None)?
        };
        let inner = RafxShaderModuleVulkanInner {
            device_context: device_context.clone(),
            shader_module,
        };

        Ok(RafxShaderModuleVulkan {
            inner: Arc::new(inner),
        })
    }

    pub fn vk_shader_module(&self) -> vk::ShaderModule {
        self.inner.shader_module
    }
}

impl Into<RafxShaderModule> for RafxShaderModuleVulkan {
    fn into(self) -> RafxShaderModule {
        RafxShaderModule::Vk(self)
    }
}
