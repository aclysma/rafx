#[cfg(feature = "rafx-metal")]
use crate::metal::{RafxDescriptorSetArrayMetal, RafxDescriptorSetHandleMetal};
use crate::vulkan::{RafxDescriptorSetArrayVulkan, RafxDescriptorSetHandleVulkan};
use crate::{RafxDescriptorUpdate, RafxResult, RafxRootSignature};

#[derive(Clone, Debug)]
pub enum RafxDescriptorSetHandle {
    Vk(RafxDescriptorSetHandleVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxDescriptorSetHandleMetal),
}

impl RafxDescriptorSetHandle {
    pub fn vk_descriptor_set_handle(&self) -> Option<&RafxDescriptorSetHandleVulkan> {
        match self {
            RafxDescriptorSetHandle::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxDescriptorSetHandle::Metal(_inner) => None,
        }
    }

    #[cfg(feature = "rafx-metal")]
    pub fn metal_descriptor_set_handle(&self) -> Option<&RafxDescriptorSetHandleMetal> {
        match self {
            RafxDescriptorSetHandle::Vk(_inner) => None,
            RafxDescriptorSetHandle::Metal(inner) => Some(inner),
        }
    }
}

#[derive(Debug)]
pub enum RafxDescriptorSetArray {
    Vk(RafxDescriptorSetArrayVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxDescriptorSetArrayMetal),
}

impl RafxDescriptorSetArray {
    pub fn handle(
        &self,
        index: u32,
    ) -> Option<RafxDescriptorSetHandle> {
        Some(match self {
            RafxDescriptorSetArray::Vk(inner) => RafxDescriptorSetHandle::Vk(inner.handle(index)?),
            #[cfg(feature = "rafx-metal")]
            RafxDescriptorSetArray::Metal(_inner) => unimplemented!(),
        })
    }

    pub fn root_signature(&self) -> &RafxRootSignature {
        match self {
            RafxDescriptorSetArray::Vk(inner) => inner.root_signature(),
            #[cfg(feature = "rafx-metal")]
            RafxDescriptorSetArray::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn update_descriptor_set(
        &mut self,
        params: &[RafxDescriptorUpdate],
    ) -> RafxResult<()> {
        match self {
            RafxDescriptorSetArray::Vk(inner) => inner.update_descriptor_set(params),
            #[cfg(feature = "rafx-metal")]
            RafxDescriptorSetArray::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn queue_descriptor_set_update(
        &mut self,
        update: &RafxDescriptorUpdate,
    ) -> RafxResult<()> {
        match self {
            RafxDescriptorSetArray::Vk(inner) => inner.queue_descriptor_set_update(update),
            #[cfg(feature = "rafx-metal")]
            RafxDescriptorSetArray::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn flush_descriptor_set_updates(&mut self) -> RafxResult<()> {
        match self {
            RafxDescriptorSetArray::Vk(inner) => inner.flush_descriptor_set_updates(),
            #[cfg(feature = "rafx-metal")]
            RafxDescriptorSetArray::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn vk_descriptor_set_array(&self) -> Option<&RafxDescriptorSetArrayVulkan> {
        match self {
            RafxDescriptorSetArray::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxDescriptorSetArray::Metal(_inner) => None,
        }
    }

    #[cfg(feature = "rafx-metal")]
    pub fn metal_descriptor_set_array(&self) -> Option<&RafxDescriptorSetArrayMetal> {
        match self {
            RafxDescriptorSetArray::Vk(_inner) => None,
            RafxDescriptorSetArray::Metal(inner) => Some(inner),
        }
    }
}
