#[cfg(feature = "rafx-metal")]
use crate::metal::{RafxDescriptorSetArrayMetal, RafxDescriptorSetHandleMetal};
#[cfg(feature = "rafx-vulkan")]
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
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxDescriptorSetHandleVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxDescriptorSetHandleMetal),
}

impl RafxDescriptorSetHandle {
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_descriptor_set_handle(&self) -> Option<&RafxDescriptorSetHandleVulkan> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDescriptorSetHandle::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxDescriptorSetHandle::Metal(_inner) => None,
        }
    }

    #[cfg(feature = "rafx-metal")]
    pub fn metal_descriptor_set_handle(&self) -> Option<&RafxDescriptorSetHandleMetal> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDescriptorSetHandle::Vk(_inner) => None,
            #[cfg(feature = "rafx-metal")]
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
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxDescriptorSetArrayVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxDescriptorSetArrayMetal),
}

impl RafxDescriptorSetArray {
    /// Create a lightweight, opaque pointer to a particular set in the array. This pointer can only
    /// be used for binding the given set in a command buffer.
    pub fn handle(
        &self,
        index: u32,
    ) -> Option<RafxDescriptorSetHandle> {
        Some(match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDescriptorSetArray::Vk(inner) => RafxDescriptorSetHandle::Vk(inner.handle(index)?),
            #[cfg(feature = "rafx-metal")]
            RafxDescriptorSetArray::Metal(inner) => {
                RafxDescriptorSetHandle::Metal(inner.handle(index)?)
            }
        })
    }

    /// Get the root signature that this descriptor set is created from
    pub fn root_signature(&self) -> &RafxRootSignature {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDescriptorSetArray::Vk(inner) => inner.root_signature(),
            #[cfg(feature = "rafx-metal")]
            RafxDescriptorSetArray::Metal(inner) => inner.root_signature(),
        }
    }

    /// Update one or more descriptor sets with new values. This is the same as calling
    /// queue_descriptor_set_update, followed by flush_descriptor_set_updates
    pub fn update_descriptor_set(
        &mut self,
        params: &[RafxDescriptorUpdate],
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDescriptorSetArray::Vk(inner) => inner.update_descriptor_set(params),
            #[cfg(feature = "rafx-metal")]
            RafxDescriptorSetArray::Metal(inner) => inner.update_descriptor_set(params),
        }
    }

    /// Update a CPU-only copy of the descriptor set, but does not apply the write to the descriptor
    /// set until flush_descriptor_set_updates() is called.
    ///
    /// The main reason for allowing queueing/flushing in separate calls is to help calling code
    /// avoid borrow-checking difficulties.
    pub fn queue_descriptor_set_update(
        &mut self,
        update: &RafxDescriptorUpdate,
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDescriptorSetArray::Vk(inner) => inner.queue_descriptor_set_update(update),
            #[cfg(feature = "rafx-metal")]
            RafxDescriptorSetArray::Metal(inner) => inner.queue_descriptor_set_update(update),
        }
    }

    /// Flush all queued descriptor set writes
    pub fn flush_descriptor_set_updates(&mut self) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDescriptorSetArray::Vk(inner) => inner.flush_descriptor_set_updates(),
            #[cfg(feature = "rafx-metal")]
            RafxDescriptorSetArray::Metal(inner) => inner.flush_descriptor_set_updates(),
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_descriptor_set_array(&self) -> Option<&RafxDescriptorSetArrayVulkan> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDescriptorSetArray::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxDescriptorSetArray::Metal(_inner) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_descriptor_set_array(&self) -> Option<&RafxDescriptorSetArrayMetal> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxDescriptorSetArray::Vk(_inner) => None,
            #[cfg(feature = "rafx-metal")]
            RafxDescriptorSetArray::Metal(inner) => Some(inner),
        }
    }
}
