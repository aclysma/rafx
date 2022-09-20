use crate::RafxResult;
use ash::vk;
use std::sync::{Arc, Mutex};

struct RafxDescriptorHeapPoolConfig {
    pool_flags: vk::DescriptorPoolCreateFlags,
    descriptor_sets: u32,
    samplers: u32,
    combined_image_samplers: u32,
    sampled_images: u32,
    storage_images: u32,
    uniform_texel_buffers: u32,
    storage_texel_buffers: u32,
    uniform_buffers: u32,
    storage_buffers: u32,
    dynamic_uniform_buffers: u32,
    dynamic_storage_buffers: u32,
    input_attachments: u32,
}

impl Default for RafxDescriptorHeapPoolConfig {
    fn default() -> Self {
        RafxDescriptorHeapPoolConfig {
            pool_flags: vk::DescriptorPoolCreateFlags::empty(),
            descriptor_sets: 8192,
            samplers: 1024,
            combined_image_samplers: 0,
            sampled_images: 8192,
            storage_images: 1024,
            uniform_texel_buffers: 1024,
            storage_texel_buffers: 1024,
            uniform_buffers: 8192,
            storage_buffers: 1024,
            dynamic_uniform_buffers: 1024,
            dynamic_storage_buffers: 0,
            input_attachments: 0,
        }
    }
}

impl RafxDescriptorHeapPoolConfig {
    fn create_pool(
        &self,
        device: &ash::Device,
    ) -> RafxResult<vk::DescriptorPool> {
        let mut pool_sizes = Vec::with_capacity(16);

        fn add_if_not_zero(
            pool_sizes: &mut Vec<vk::DescriptorPoolSize>,
            ty: vk::DescriptorType,
            descriptor_count: u32,
        ) {
            if descriptor_count != 0 {
                pool_sizes.push(vk::DescriptorPoolSize {
                    ty,
                    descriptor_count,
                });
            }
        }

        #[rustfmt::skip]
        {
            add_if_not_zero(&mut pool_sizes, vk::DescriptorType::SAMPLER, self.samplers);
            add_if_not_zero(&mut pool_sizes, vk::DescriptorType::COMBINED_IMAGE_SAMPLER, self.combined_image_samplers);
            add_if_not_zero(&mut pool_sizes, vk::DescriptorType::SAMPLED_IMAGE, self.sampled_images);
            add_if_not_zero(&mut pool_sizes, vk::DescriptorType::STORAGE_IMAGE, self.storage_images);
            add_if_not_zero(&mut pool_sizes, vk::DescriptorType::UNIFORM_TEXEL_BUFFER, self.uniform_texel_buffers);
            add_if_not_zero(&mut pool_sizes, vk::DescriptorType::STORAGE_TEXEL_BUFFER, self.storage_texel_buffers);
            add_if_not_zero(&mut pool_sizes, vk::DescriptorType::UNIFORM_BUFFER, self.uniform_buffers);
            add_if_not_zero(&mut pool_sizes, vk::DescriptorType::STORAGE_BUFFER, self.storage_buffers);
            add_if_not_zero(&mut pool_sizes, vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC, self.dynamic_uniform_buffers);
            add_if_not_zero(&mut pool_sizes, vk::DescriptorType::STORAGE_BUFFER_DYNAMIC, self.dynamic_storage_buffers);
            add_if_not_zero(&mut pool_sizes, vk::DescriptorType::INPUT_ATTACHMENT, self.input_attachments);
        };

        unsafe {
            Ok(device.create_descriptor_pool(
                &*vk::DescriptorPoolCreateInfo::builder()
                    .flags(self.pool_flags)
                    .max_sets(self.descriptor_sets)
                    .pool_sizes(&pool_sizes),
                None,
            )?)
        }
    }
}

struct RafxDescriptorHeapVulkanInner {
    heap_pool_config: RafxDescriptorHeapPoolConfig,
    pools: Vec<vk::DescriptorPool>,
}

impl RafxDescriptorHeapVulkanInner {
    fn clear_pools(
        &mut self,
        device: &ash::Device,
    ) {
        for &pool in &self.pools {
            unsafe {
                device.destroy_descriptor_pool(pool, None);
            }
        }

        self.pools.clear();
    }
}

impl Drop for RafxDescriptorHeapVulkanInner {
    fn drop(&mut self) {
        // Assert that everything was destroyed. (We can't do it automatically since we don't have
        // a reference to the device)
        assert!(self.pools.is_empty());
    }
}

// This is an endlessly growing descriptor pools. New pools are allocated in large chunks as needed.
// It also takes locks on every operation. So it's better to allocate large chunks of descriptors
// and pool/reuse them.
#[derive(Clone)]
pub(crate) struct RafxDescriptorHeapVulkan {
    inner: Arc<Mutex<RafxDescriptorHeapVulkanInner>>,
}

impl RafxDescriptorHeapVulkan {
    pub(crate) fn new(device: &ash::Device) -> RafxResult<Self> {
        let heap_pool_config = RafxDescriptorHeapPoolConfig::default();
        let pool = heap_pool_config.create_pool(device)?;

        let inner = RafxDescriptorHeapVulkanInner {
            heap_pool_config,
            pools: vec![pool],
        };

        Ok(RafxDescriptorHeapVulkan {
            inner: Arc::new(Mutex::new(inner)),
        })
    }

    pub(crate) fn clear_pools(
        &self,
        device: &ash::Device,
    ) {
        self.inner.lock().unwrap().clear_pools(device);
    }

    pub(crate) fn allocate_descriptor_sets(
        &self,
        device: &ash::Device,
        set_layouts: &[vk::DescriptorSetLayout],
    ) -> RafxResult<Vec<vk::DescriptorSet>> {
        let mut heap = self.inner.lock().unwrap();

        let mut allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .set_layouts(set_layouts)
            .build();

        // Heap might have been cleared
        if !heap.pools.is_empty() {
            let pool = *heap.pools.last().unwrap();
            allocate_info.descriptor_pool = pool;

            let result = unsafe { device.allocate_descriptor_sets(&allocate_info) };

            // If successful bail, otherwise allocate a new pool below
            if let Ok(result) = result {
                return Ok(result);
            }
        }

        // We either didn't have any pools, or assume the pool wasn't large enough. Create a new
        // pool and try again
        let new_pool = heap.heap_pool_config.create_pool(device)?;
        heap.pools.push(new_pool);

        let pool = *heap.pools.last().unwrap();
        allocate_info.descriptor_pool = pool;
        Ok(unsafe { device.allocate_descriptor_sets(&allocate_info)? })
    }
}
