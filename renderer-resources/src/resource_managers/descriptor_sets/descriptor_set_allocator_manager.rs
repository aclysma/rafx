use std::sync::{Mutex, Arc};
use super::DescriptorSetAllocator;
use crossbeam_channel::{Sender, Receiver};
use renderer_shell_vulkan::VkDeviceContext;
use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use ash::prelude::VkResult;

// This holds the allocator and the frame on which it was "borrowed" from the allocator manager
struct DescriptorSetAllocatorRefInner {
    allocator: Box<DescriptorSetAllocator>,
    checkout_frame: u64,
}

// A borrowed allocator that returns itself when it is dropped. It is expected that these borrows
// are short (i.e. within a single frame). Holding an allocator over multiple frames can delay
// releasing descriptor sets that have been dropped.
pub struct DescriptorSetAllocatorRef {
    // This should never be None. We always allocate this to a non-none value and we don't clear
    // it until the drop handler
    allocator: Option<DescriptorSetAllocatorRefInner>,
    drop_tx: Sender<DescriptorSetAllocatorRefInner>,
}

impl DescriptorSetAllocatorRef {
    fn new(
        allocator: DescriptorSetAllocatorRefInner,
        drop_tx: Sender<DescriptorSetAllocatorRefInner>,
    ) -> Self {
        DescriptorSetAllocatorRef {
            allocator: Some(allocator),
            drop_tx,
        }
    }
}

impl Deref for DescriptorSetAllocatorRef {
    type Target = DescriptorSetAllocator;

    fn deref(&self) -> &Self::Target {
        &self.allocator.as_ref().unwrap().allocator
    }
}

impl DerefMut for DescriptorSetAllocatorRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.allocator.as_mut().unwrap().allocator
    }
}

impl Drop for DescriptorSetAllocatorRef {
    fn drop(&mut self) {
        let mut allocator = self.allocator.take().unwrap();
        allocator.allocator.flush_changes().unwrap();
        self.drop_tx.send(allocator).unwrap();
    }
}

// A pool of descriptor set allocators. The allocators themselves contain pools for descriptor set
// layouts.
pub struct DescriptorSetAllocatorManagerInner {
    device_context: VkDeviceContext,
    allocators: Mutex<VecDeque<Box<DescriptorSetAllocator>>>,
    drop_tx: Sender<DescriptorSetAllocatorRefInner>,
    drop_rx: Receiver<DescriptorSetAllocatorRefInner>,
    frame_index: AtomicU64,
}

impl DescriptorSetAllocatorManagerInner {
    fn new(device_context: VkDeviceContext) -> Self {
        let (drop_tx, drop_rx) = crossbeam_channel::unbounded();

        DescriptorSetAllocatorManagerInner {
            device_context,
            allocators: Default::default(),
            drop_tx,
            drop_rx,
            frame_index: AtomicU64::new(0),
        }
    }

    // Internally used to pull any dropped allocators back into the pool. If on_frame_complete
    // was called on the manager since it was borrowed, call it on the allocator. This lets us
    // drain any drops and schedule them for deletion after MAX_FRAMES_IN_FLIGHT passes. We only
    // call it once, even if several frames have passed, because we have no way of knowing if
    // descriptors were dropped recently or multiple frames ago.
    fn drain_drop_rx(
        drop_rx: &Receiver<DescriptorSetAllocatorRefInner>,
        allocators: &mut VecDeque<Box<DescriptorSetAllocator>>,
        frame_index: u64,
    ) {
        for mut allocator in drop_rx.try_iter() {
            if allocator.checkout_frame < frame_index {
                allocator.allocator.on_frame_complete();
            }

            if frame_index - allocator.checkout_frame > 1 {
                // Holding DescriptorSetAllocatorRefs for more than a frame will delay releasing
                // unused descriptors
                log::warn!("A DescriptorSetAllocatorRef was held for more than one frame.");
            }

            allocators.push_back(allocator.allocator);
        }
    }

    pub fn get_allocator(&self) -> DescriptorSetAllocatorRef {
        let frame_index = self.frame_index.load(Ordering::Relaxed);
        let allocator = {
            let mut allocators = self.allocators.lock().unwrap();
            Self::drain_drop_rx(&self.drop_rx, &mut *allocators, frame_index);

            allocators
                .pop_front()
                .map(|allocator| DescriptorSetAllocatorRefInner {
                    allocator,
                    checkout_frame: frame_index,
                })
        };

        let allocator = allocator.unwrap_or_else(|| {
            let allocator = Box::new(DescriptorSetAllocator::new(&self.device_context));

            DescriptorSetAllocatorRefInner {
                allocator,
                checkout_frame: frame_index,
            }
        });

        DescriptorSetAllocatorRef::new(allocator, self.drop_tx.clone())
    }

    pub fn on_frame_complete(&self) {
        let frame_index = self.frame_index.fetch_add(1, Ordering::Relaxed);
        let mut allocators = self.allocators.lock().unwrap();

        Self::drain_drop_rx(&self.drop_rx, &mut *allocators, frame_index);

        for allocator in allocators.iter_mut() {
            allocator.on_frame_complete();
        }
    }

    fn destroy(&self) -> VkResult<()> {
        let frame_index = self.frame_index.load(Ordering::Relaxed);
        let mut allocators = self.allocators.lock().unwrap();

        Self::drain_drop_rx(&self.drop_rx, &mut *allocators, frame_index);

        for mut allocator in allocators.drain(..).into_iter() {
            allocator.destroy()?;
        }

        Ok(())
    }
}

pub struct DescriptorSetAllocatorProvider {
    inner: Arc<DescriptorSetAllocatorManagerInner>,
}

impl DescriptorSetAllocatorProvider {
    pub fn get_allocator(&self) -> DescriptorSetAllocatorRef {
        self.inner.get_allocator()
    }
}

pub struct DescriptorSetAllocatorManager {
    inner: Arc<DescriptorSetAllocatorManagerInner>,
}

impl DescriptorSetAllocatorManager {
    pub fn new(device_context: &VkDeviceContext) -> Self {
        DescriptorSetAllocatorManager {
            inner: Arc::new(DescriptorSetAllocatorManagerInner::new(
                device_context.clone(),
            )),
        }
    }

    pub fn create_allocator_provider(&self) -> DescriptorSetAllocatorProvider {
        DescriptorSetAllocatorProvider {
            inner: self.inner.clone(),
        }
    }

    pub fn get_allocator(&self) -> DescriptorSetAllocatorRef {
        self.inner.get_allocator()
    }

    pub fn on_frame_complete(&self) {
        self.inner.on_frame_complete();
    }

    pub fn destroy(&mut self) -> VkResult<()> {
        self.inner.destroy()
    }
}
