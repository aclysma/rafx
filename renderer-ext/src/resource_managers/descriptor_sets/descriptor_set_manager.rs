use ash::vk;
use crate::resource_managers::resource_lookup::{
    ResourceHash, DescriptorSetLayoutResource, ResourceLookupSet,
};
use renderer_shell_vulkan::VkDeviceContext;
use fnv::FnvHashMap;
use super::RegisteredDescriptorSetPool;
use super::{FrameInFlightIndex, DescriptorSetArc, MAX_FRAMES_IN_FLIGHT};
use super::DescriptorSetWriteSet;
use ash::prelude::VkResult;
use crate::resource_managers::{DynDescriptorSet, DynPassMaterialInstance, DynMaterialInstance, ResourceArc};
use crate::resource_managers::asset_lookup::{
    LoadedMaterialPass, LoadedAssetLookupSet, LoadedMaterialInstance, LoadedMaterial,
};
use crate::pipeline_description as dsc;

#[derive(Debug)]
pub struct RegisteredDescriptorSetPoolMetrics {
    pub hash: ResourceHash,
    pub allocated_count: usize,
}

#[derive(Debug)]
pub struct RegisteredDescriptorSetPoolManagerMetrics {
    pub pools: Vec<RegisteredDescriptorSetPoolMetrics>,
}

pub struct RegisteredDescriptorSetPoolManager {
    device_context: VkDeviceContext,
    pools: FnvHashMap<ResourceHash, RegisteredDescriptorSetPool>,

    // This index represents the set of resources that will be written to when update() is called.
    frame_in_flight_index: FrameInFlightIndex,
}

impl RegisteredDescriptorSetPoolManager {
    pub fn new(device_context: &VkDeviceContext) -> Self {
        RegisteredDescriptorSetPoolManager {
            device_context: device_context.clone(),
            pools: Default::default(),
            frame_in_flight_index: 0,
        }
    }

    pub fn metrics(&self) -> RegisteredDescriptorSetPoolManagerMetrics {
        let mut registered_descriptor_sets_stats = Vec::with_capacity(self.pools.len());
        for (hash, value) in &self.pools {
            let pool_stats = RegisteredDescriptorSetPoolMetrics {
                hash: *hash,
                allocated_count: value.slab.allocated_count(),
            };
            registered_descriptor_sets_stats.push(pool_stats);
        }

        RegisteredDescriptorSetPoolManagerMetrics {
            pools: registered_descriptor_sets_stats,
        }
    }

    pub fn descriptor_set_for_cpu_write(
        &self,
        descriptor_set_arc: &DescriptorSetArc,
    ) -> vk::DescriptorSet {
        descriptor_set_arc.inner.descriptor_sets_per_frame[self.frame_in_flight_index as usize]
    }

    pub fn descriptor_set_for_gpu_read(
        &self,
        descriptor_set_arc: &DescriptorSetArc,
    ) -> vk::DescriptorSet {
        let gpu_read_frame_in_flight_index = if self.frame_in_flight_index == 0 {
            MAX_FRAMES_IN_FLIGHT
        } else {
            self.frame_in_flight_index as usize - 1
        };

        //println!("use index {}", gpu_read_frame_in_flight_index);
        descriptor_set_arc.inner.descriptor_sets_per_frame[gpu_read_frame_in_flight_index]
        //self.descriptor_set_for_cpu_write(descriptor_set_arc)
    }

    pub fn insert(
        &mut self,
        descriptor_set_layout_def: &dsc::DescriptorSetLayout,
        descriptor_set_layout: ResourceArc<DescriptorSetLayoutResource>,
        write_set: DescriptorSetWriteSet,
    ) -> VkResult<DescriptorSetArc> {
        let hash = ResourceHash::from_key(descriptor_set_layout_def);
        let device_context = self.device_context.clone();
        let pool = self.pools.entry(hash).or_insert_with(|| {
            RegisteredDescriptorSetPool::new(
                &device_context,
                descriptor_set_layout_def,
                descriptor_set_layout,
            )
        });

        pool.insert(&self.device_context, write_set, self.frame_in_flight_index)
    }

    pub fn update(&mut self) {
        // Schedule any descriptor set/buffer changes that occurred since the previous update.
        //
        for pool in self.pools.values_mut() {
            pool.schedule_changes(&self.device_context, self.frame_in_flight_index);
        }

        // Now process drops and flush writes to GPU
        for pool in self.pools.values_mut() {
            pool.flush_changes(&self.device_context, self.frame_in_flight_index);
        }

        // Bump frame in flight index
        self.frame_in_flight_index =
            super::add_to_frame_in_flight_index(self.frame_in_flight_index, 1);
    }

    pub fn destroy(&mut self) {
        for (hash, pool) in &mut self.pools {
            pool.destroy(&self.device_context);
        }

        self.pools.clear();
    }

    //TODO: Is creating and immediately modifying causing multiple writes?
    fn do_create_dyn_descriptor_set(
        &mut self,
        write_set: DescriptorSetWriteSet,
        descriptor_set_layout_def: &dsc::DescriptorSetLayout,
        descriptor_set_layout: ResourceArc<DescriptorSetLayoutResource>,
    ) -> VkResult<DynDescriptorSet> {
        // Get or create the pool for the layout
        let hash = ResourceHash::from_key(descriptor_set_layout_def);
        let device_context = self.device_context.clone();
        let pool = self.pools.entry(hash).or_insert_with(|| {
            RegisteredDescriptorSetPool::new(
                &device_context,
                descriptor_set_layout_def,
                descriptor_set_layout,
            )
        });

        // Allocate a descriptor set
        let descriptor_set = pool.insert(
            &self.device_context,
            write_set.clone(),
            self.frame_in_flight_index,
        )?;

        // Create the DynDescriptorSet
        let dyn_descriptor_set = DynDescriptorSet::new(
            write_set,
            descriptor_set,
            pool.write_set_tx.clone(),
        );

        Ok(dyn_descriptor_set)
    }

    pub fn create_dyn_descriptor_set_uninitialized(
        &mut self,
        descriptor_set_layout_def: &dsc::DescriptorSetLayout,
        descriptor_set_layout: ResourceArc<DescriptorSetLayoutResource>,
    ) -> VkResult<DynDescriptorSet> {
        let write_set = super::create_uninitialized_write_set_for_layout(descriptor_set_layout_def);
        self.do_create_dyn_descriptor_set(
            write_set,
            descriptor_set_layout_def,
            descriptor_set_layout,
        )
    }

    pub fn create_dyn_pass_material_instance_uninitialized(
        &mut self,
        pass: &LoadedMaterialPass,
        loaded_assets: &LoadedAssetLookupSet,
    ) -> VkResult<DynPassMaterialInstance> {
        let mut dyn_descriptor_sets = Vec::with_capacity(pass.descriptor_set_layouts.len());

        let layout_defs = &pass
            .pipeline_create_data
            .pipeline_layout_def
            .descriptor_set_layouts;
        for (layout_def, layout) in layout_defs.iter().zip(&pass.descriptor_set_layouts) {
            let dyn_descriptor_set =
                self.create_dyn_descriptor_set_uninitialized(layout_def, layout.clone())?;
            dyn_descriptor_sets.push(dyn_descriptor_set);
        }

        let dyn_pass_material_instance =
            DynPassMaterialInstance::new(dyn_descriptor_sets, pass.pass_slot_name_lookup.clone());
        Ok(dyn_pass_material_instance)
    }

    pub fn create_dyn_pass_material_instance_from_asset(
        &mut self,
        pass: &LoadedMaterialPass,
        material_instance: &LoadedMaterialInstance,
        loaded_assets: &LoadedAssetLookupSet,
        resources: &mut ResourceLookupSet,
    ) -> VkResult<DynPassMaterialInstance> {
        let write_sets = super::create_write_sets_for_material_instance_pass(
            pass,
            &material_instance.slot_assignments,
            loaded_assets,
            resources,
        )?;

        let mut dyn_descriptor_sets = Vec::with_capacity(write_sets.len());

        for (layout_index, write_set) in write_sets.into_iter().enumerate() {
            let layout = &pass.descriptor_set_layouts[layout_index];
            let layout_def = &pass
                .pipeline_create_data
                .pipeline_layout_def
                .descriptor_set_layouts[layout_index];

            let dyn_descriptor_set =
                self.do_create_dyn_descriptor_set(write_set, layout_def, layout.clone())?;
            dyn_descriptor_sets.push(dyn_descriptor_set);
        }

        let dyn_pass_material_instance =
            DynPassMaterialInstance::new(dyn_descriptor_sets, pass.pass_slot_name_lookup.clone());
        Ok(dyn_pass_material_instance)
    }

    pub fn create_dyn_material_instance_uninitialized(
        &mut self,
        material: &LoadedMaterial,
        loaded_assets: &LoadedAssetLookupSet,
    ) -> VkResult<DynMaterialInstance> {
        let mut passes = Vec::with_capacity(material.passes.len());
        for pass in &material.passes {
            let dyn_pass_material_instance =
                self.create_dyn_pass_material_instance_uninitialized(pass, loaded_assets)?;
            passes.push(dyn_pass_material_instance);
        }

        Ok(DynMaterialInstance::new(passes))
    }

    pub fn create_dyn_material_instance_from_asset(
        &mut self,
        material: &LoadedMaterial,
        material_instance: &LoadedMaterialInstance,
        loaded_assets: &LoadedAssetLookupSet,
        resources: &mut ResourceLookupSet,
    ) -> VkResult<DynMaterialInstance> {
        let mut passes = Vec::with_capacity(material.passes.len());
        for pass in &material.passes {
            let dyn_pass_material_instance = self.create_dyn_pass_material_instance_from_asset(
                pass,
                material_instance,
                loaded_assets,
                resources,
            )?;
            passes.push(dyn_pass_material_instance);
        }

        Ok(DynMaterialInstance::new(passes))
    }
}
