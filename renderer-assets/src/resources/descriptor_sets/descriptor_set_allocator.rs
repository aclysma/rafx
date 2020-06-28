use crate::resources::resource_lookup::{ResourceHash, DescriptorSetLayoutResource};
use renderer_shell_vulkan::VkDeviceContext;
use fnv::FnvHashMap;
use super::ManagedDescriptorSetPool;
use super::{FrameInFlightIndex, DescriptorSetArc};
use super::DescriptorSetWriteSet;
use ash::prelude::VkResult;
use crate::resources::{
    DynDescriptorSet, DynPassMaterialInstance, DynMaterialInstance, ResourceArc,
};
use crate::assets::{MaterialPass, MaterialInstanceAsset, MaterialAsset};

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
    device_context: VkDeviceContext,
    pools: FnvHashMap<ResourceHash, ManagedDescriptorSetPool>,

    // This index represents the set of resources that will be written to when update() is called.
    frame_in_flight_index: FrameInFlightIndex,
}

impl DescriptorSetAllocator {
    pub fn new(device_context: &VkDeviceContext) -> Self {
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

    pub fn flush_changes(&mut self) -> VkResult<()> {
        // Now process drops and flush writes to GPU
        for pool in self.pools.values_mut() {
            pool.flush_changes(&self.device_context, self.frame_in_flight_index)?;
        }

        Ok(())
    }

    pub fn on_frame_complete(&mut self) {
        // Bump frame in flight index
        self.frame_in_flight_index =
            super::add_to_frame_in_flight_index(self.frame_in_flight_index, 1);
    }

    pub fn destroy(&mut self) -> VkResult<()> {
        for pool in self.pools.values_mut() {
            pool.destroy(&self.device_context)?;
        }

        self.pools.clear();
        Ok(())
    }

    pub fn create_descriptor_set(
        &mut self,
        descriptor_set_layout: &ResourceArc<DescriptorSetLayoutResource>,
        write_set: DescriptorSetWriteSet,
    ) -> VkResult<DescriptorSetArc> {
        // Get or create the pool for the layout
        let hash = descriptor_set_layout.get_hash().into();
        let device_context = self.device_context.clone();
        let pool = self.pools.entry(hash).or_insert_with(|| {
            ManagedDescriptorSetPool::new(&device_context, descriptor_set_layout.clone())
        });

        // Allocate a descriptor set
        pool.insert(&self.device_context, write_set)
    }

    //TODO: Is creating and immediately modifying causing multiple writes?
    fn do_create_dyn_descriptor_set(
        &mut self,
        descriptor_set_layout: &ResourceArc<DescriptorSetLayoutResource>,
        write_set: DescriptorSetWriteSet,
    ) -> VkResult<DynDescriptorSet> {
        let descriptor_set =
            self.create_descriptor_set(descriptor_set_layout, write_set.clone())?;

        // Create the DynDescriptorSet
        let dyn_descriptor_set =
            DynDescriptorSet::new(descriptor_set_layout, descriptor_set, write_set);

        Ok(dyn_descriptor_set)
    }

    pub fn create_dyn_descriptor_set_uninitialized(
        &mut self,
        descriptor_set_layout: &ResourceArc<DescriptorSetLayoutResource>,
    ) -> VkResult<DynDescriptorSet> {
        let write_set = super::create_uninitialized_write_set_for_layout(
            &descriptor_set_layout.get_raw().descriptor_set_layout_def,
        );
        self.do_create_dyn_descriptor_set(descriptor_set_layout, write_set)
    }

    pub fn create_dyn_pass_material_instance_uninitialized(
        &mut self,
        pass: &MaterialPass,
    ) -> VkResult<DynPassMaterialInstance> {
        let mut dyn_descriptor_sets = Vec::with_capacity(pass.descriptor_set_layouts.len());

        for layout in &pass.descriptor_set_layouts {
            let dyn_descriptor_set = self.create_dyn_descriptor_set_uninitialized(layout)?;
            dyn_descriptor_sets.push(dyn_descriptor_set);
        }

        let dyn_pass_material_instance =
            DynPassMaterialInstance::new(dyn_descriptor_sets, pass.pass_slot_name_lookup.clone());
        Ok(dyn_pass_material_instance)
    }

    pub fn create_dyn_pass_material_instance_from_asset(
        &mut self,
        pass: &MaterialPass,
        write_sets: Vec<DescriptorSetWriteSet>,
    ) -> VkResult<DynPassMaterialInstance> {
        let mut dyn_descriptor_sets = Vec::with_capacity(write_sets.len());

        for (layout_index, write_set) in write_sets.into_iter().enumerate() {
            let layout = &pass.descriptor_set_layouts[layout_index];
            let dyn_descriptor_set = self.do_create_dyn_descriptor_set(layout, write_set)?;
            dyn_descriptor_sets.push(dyn_descriptor_set);
        }

        let dyn_pass_material_instance =
            DynPassMaterialInstance::new(dyn_descriptor_sets, pass.pass_slot_name_lookup.clone());
        Ok(dyn_pass_material_instance)
    }

    pub fn create_dyn_material_instance_uninitialized(
        &mut self,
        material: &MaterialAsset,
    ) -> VkResult<DynMaterialInstance> {
        let mut passes = Vec::with_capacity(material.passes.len());
        for pass in &*material.passes {
            let dyn_pass_material_instance =
                self.create_dyn_pass_material_instance_uninitialized(pass)?;
            passes.push(dyn_pass_material_instance);
        }

        Ok(DynMaterialInstance::new(passes))
    }

    pub fn create_dyn_material_instance_from_asset(
        &mut self,
        material: &MaterialAsset,
        material_instance: &MaterialInstanceAsset,
    ) -> VkResult<DynMaterialInstance> {
        let mut passes = Vec::with_capacity(material.passes.len());
        for (pass, write_sets) in material
            .passes
            .iter()
            .zip(material_instance.inner.descriptor_set_writes.clone())
        {
            let dyn_pass_material_instance =
                self.create_dyn_pass_material_instance_from_asset(pass, write_sets)?;
            passes.push(dyn_pass_material_instance);
        }

        Ok(DynMaterialInstance::new(passes))
    }
}
