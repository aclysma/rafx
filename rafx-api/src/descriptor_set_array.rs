#[cfg(feature = "rafx-metal")]
use crate::metal::{RafxDescriptorSetArrayMetal, RafxDescriptorSetHandleMetal};
use crate::vulkan::{RafxDescriptorSetArrayVulkan, RafxDescriptorSetHandleVulkan};
use crate::{RafxDescriptorUpdate, RafxResult, RafxRootSignature};

/// A lightweight handle to a specific descriptor set in a `RafxDescriptorSetArray`.
///
/// Modifying a descriptor set in a `RafxDescriptorSetArray` requires mutable access to the array.
/// However, many times in an application it is necessary to obtain and use references to
/// individual descriptor sets. These descriptor sets are not used or even accessed by the CPU, they
/// are just handles that need to be provided to the GPU.
///
/// A `RafxDescriptorSetHandle` can be used to reference descriptor sets and bind them to command
/// buffers from different threads.
///
/// This object is generally speaking optional. A single-threaded application can use
/// `RafxDescriptorSetArray` directly at any time.
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

/// Represents an array of descriptor sets.
///
/// Managing descriptor sets can be challenging and there are many strategies that may be used. So
/// a `RafxDescriptorSetArray` is intended to be allocated in blocks and pooled. This allows
/// downstream code to provide more fine-grained allocation strategies appropriate to their needs.
///
/// Higher level crates in rafx-resources provide ref-counted descriptor sets and pooling.
///
/// Once a RafxDescriptorSetArray is allocated, depending on the backend, it may remain allocated
/// for the duration of the API object, even if the descriptor set array is dropped. So rather than
/// drop them, pool and reuse them.
///
/// Descriptor sets are like pointers to GPU memory. A command buffer can bind a descriptor set,
/// meaning that other command may access resources that the descriptor set references.
///
/// Once a command buffer using a descriptor set has been submitted, it must not be modified until
/// the command buffer is finished executing. Fine-grained synchronization primitives allow this
/// restriction to be loosened.
///
/// **Using an incorrectly configured descriptor set can result in undefined behavior. In practice,
/// this can include GPU hangs, driver crashes, and kernel panics**.
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
