use super::DescriptorSetWriteSet;
use super::ManagedDescriptorSetPool;
use super::{DescriptorSetArc, FrameInFlightIndex};
use crate::resources::resource_lookup::{DescriptorSetLayoutResource, ResourceHash};
use crate::resources::{DynDescriptorSet, ResourceArc};
use fnv::FnvHashMap;
use rafx_api::{RafxDeviceContext, RafxResult};

#[derive(Debug)]
pub struct DescriptorSetPoolMetrics {
    pub hash: ResourceHash,
    pub allocated_count: usize,
}

#[derive(Debug)]
pub struct DescriptorSetAllocatorMetrics {
    pub pools: Vec<DescriptorSetPoolMetrics>,
}

pub struct DescriptorSetAllocator {
    device_context: RafxDeviceContext,
    pools: FnvHashMap<ResourceHash, ManagedDescriptorSetPool>,

    // This index represents the set of resources that will be written to when update() is called.
    frame_in_flight_index: FrameInFlightIndex,
}

impl DescriptorSetAllocator {
    pub fn new(device_context: &RafxDeviceContext) -> Self {
        DescriptorSetAllocator {
            device_context: device_context.clone(),
            pools: Default::default(),
            frame_in_flight_index: 0,
        }
    }

    pub fn metrics(&self) -> DescriptorSetAllocatorMetrics {
        let mut registered_descriptor_sets_stats = Vec::with_capacity(self.pools.len());
        for (hash, value) in &self.pools {
            let pool_stats = DescriptorSetPoolMetrics {
                hash: *hash,
                allocated_count: value.slab.allocated_count(),
            };
            registered_descriptor_sets_stats.push(pool_stats);
        }

        DescriptorSetAllocatorMetrics {
            pools: registered_descriptor_sets_stats,
        }
    }

    #[profiling::function]
    pub fn flush_changes(&mut self) -> RafxResult<()> {
        // Now process drops and flush writes to GPU
        for pool in self.pools.values_mut() {
            pool.flush_changes(self.frame_in_flight_index)?;
        }

        Ok(())
    }

    #[profiling::function]
    pub fn on_frame_complete(&mut self) {
        // Bump frame in flight index
        self.frame_in_flight_index =
            super::add_to_frame_in_flight_index(self.frame_in_flight_index, 1);
    }

    pub fn destroy(&mut self) -> RafxResult<()> {
        for pool in self.pools.values_mut() {
            pool.destroy()?;
        }

        self.pools.clear();
        Ok(())
    }

    //TODO: Is creating and immediately modifying causing multiple writes?
    fn do_create_dyn_descriptor_set(
        &mut self,
        descriptor_set_layout: &ResourceArc<DescriptorSetLayoutResource>,
        write_set: DescriptorSetWriteSet,
    ) -> RafxResult<DynDescriptorSet> {
        let descriptor_set =
            self.create_descriptor_set_with_writes(descriptor_set_layout, write_set.clone())?;

        // Create the DynDescriptorSet
        let dyn_descriptor_set =
            DynDescriptorSet::new(descriptor_set_layout, descriptor_set, write_set);

        Ok(dyn_descriptor_set)
    }

    pub fn create_descriptor_set_with_writes(
        &mut self,
        descriptor_set_layout: &ResourceArc<DescriptorSetLayoutResource>,
        write_set: DescriptorSetWriteSet,
    ) -> RafxResult<DescriptorSetArc> {
        // Get or create the pool for the layout

        let hash = descriptor_set_layout.get_hash().into();
        let device_context = self.device_context.clone();

        let pool = self.pools.entry(hash).or_insert_with(|| {
            ManagedDescriptorSetPool::new(&device_context, descriptor_set_layout.clone())
        });

        // Allocate a descriptor set
        pool.insert(&self.device_context, write_set)
    }

    pub fn create_descriptor_set<'a, T: DescriptorSetInitializer<'a>>(
        &mut self,
        descriptor_set_layout: &ResourceArc<DescriptorSetLayoutResource>,
        args: T,
    ) -> RafxResult<DescriptorSetArc> {
        let descriptor_set = self.create_dyn_descriptor_set_uninitialized(descriptor_set_layout)?;
        T::create_descriptor_set(self, descriptor_set, args)
    }

    pub fn create_dyn_descriptor_set_uninitialized(
        &mut self,
        descriptor_set_layout: &ResourceArc<DescriptorSetLayoutResource>,
    ) -> RafxResult<DynDescriptorSet> {
        let write_set = super::create_uninitialized_write_set_for_layout(
            &descriptor_set_layout.get_raw().descriptor_set_layout_def,
        );
        self.do_create_dyn_descriptor_set(descriptor_set_layout, write_set)
    }

    pub fn create_dyn_descriptor_set<'a, T: DescriptorSetInitializer<'a>>(
        &mut self,
        descriptor_set_layout: &ResourceArc<DescriptorSetLayoutResource>,
        args: T,
    ) -> RafxResult<<T as DescriptorSetInitializer<'a>>::Output> {
        Ok(T::create_dyn_descriptor_set(
            self.create_dyn_descriptor_set_uninitialized(descriptor_set_layout)?,
            args,
        ))
    }
}

// The lifetime here is required so that implementations of it can bind to it
pub trait DescriptorSetInitializer<'a> {
    type Output;

    fn create_dyn_descriptor_set(
        descriptor_set: DynDescriptorSet,
        args: Self,
    ) -> Self::Output;
    fn create_descriptor_set(
        descriptor_set_allocator: &mut DescriptorSetAllocator,
        descriptor_set: DynDescriptorSet,
        args: Self,
    ) -> RafxResult<DescriptorSetArc>;
}
