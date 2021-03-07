use crate::assets::ImageAssetData;
use crate::assets::{BufferAsset, ImageAsset, MaterialAsset};
use crate::{
    AssetLookup, AssetTypeHandler, AssetTypeHandlerFactory, BufferAssetData, GenericLoader,
    MaterialInstanceSlotAssignment, UploadQueueConfig,
};
use distill::loader::handle::Handle;
use rafx_framework::{
    DescriptorSetAllocatorMetrics, DescriptorSetAllocatorProvider, DescriptorSetAllocatorRef,
    DescriptorSetLayoutResource, DescriptorSetWriteSet, DynResourceAllocatorSet,
    GraphicsPipelineCache, MaterialPass, MaterialPassResource, ResourceArc, SlotNameLookup,
};

use super::upload::UploadManager;
use crate::assets::buffer::BufferAssetTypeHandler;
use crate::assets::compute_pipeline::ComputePipelineAssetTypeHandler;
use crate::assets::graphics_pipeline::{
    MaterialAssetTypeHandler, MaterialInstanceAssetTypeHandler, SamplerAssetTypeHandler,
};
use crate::assets::image::ImageAssetTypeHandler;
use crate::assets::shader::ShaderAssetTypeHandler;
use crate::distill_impl::AssetResource;
use distill::loader::handle::AssetHandle;
use distill::loader::storage::LoadStatus;
use fnv::FnvHashMap;
use rafx_api::{RafxDeviceContext, RafxQueue, RafxResult};
use rafx_framework::descriptor_sets::{
    DescriptorSetElementKey, DescriptorSetWriteElementBuffer, DescriptorSetWriteElementBufferData,
    DescriptorSetWriteElementImage,
};
use rafx_framework::nodes::RenderRegistry;
use rafx_framework::DescriptorSetAllocator;
use rafx_framework::DynCommandPoolAllocator;
use rafx_framework::DynResourceAllocatorSetProvider;
use rafx_framework::ResourceLookupSet;
use rafx_framework::{ResourceManager, ResourceManagerMetrics};
use std::any::TypeId;
use std::sync::Arc;

#[derive(Debug)]
pub struct AssetManagerMetrics {
    pub resource_manager_metrics: ResourceManagerMetrics,
    pub material_instance_descriptor_sets_metrics: DescriptorSetAllocatorMetrics,
    //TODO: Metrics per asset type
}

pub struct AssetManagerLoaders {
    pub image_loader: GenericLoader<ImageAssetData, ImageAsset>,
    pub buffer_loader: GenericLoader<BufferAssetData, BufferAsset>,
}

pub struct AssetManager {
    device_context: RafxDeviceContext,
    resource_manager: ResourceManager,
    upload_manager: UploadManager,
    material_instance_descriptor_sets: DescriptorSetAllocator,
    graphics_queue: RafxQueue,
    transfer_queue: RafxQueue,

    asset_types: FnvHashMap<TypeId, Box<dyn AssetTypeHandler>>,
    // Extremely rare that we modify asset_registration_order but we need to iterate it while
    // having a mut reference to asset manager. Better to just reallocate the vec every time we
    // register an asset type than clone every time we do a frame update.
    asset_registration_order: Arc<Vec<TypeId>>,
}

impl AssetManager {
    pub fn new(
        device_context: &RafxDeviceContext,
        render_registry: &RenderRegistry,
        upload_queue_config: UploadQueueConfig,
        graphics_queue: &RafxQueue,
        transfer_queue: &RafxQueue,
    ) -> Self {
        let resource_manager = ResourceManager::new(device_context, render_registry);

        AssetManager {
            device_context: device_context.clone(),
            resource_manager,
            upload_manager: UploadManager::new(
                device_context,
                upload_queue_config,
                graphics_queue.clone(),
                transfer_queue.clone(),
            ),
            material_instance_descriptor_sets: DescriptorSetAllocator::new(device_context),
            graphics_queue: graphics_queue.clone(),
            transfer_queue: transfer_queue.clone(),

            asset_types: Default::default(),
            asset_registration_order: Default::default(),
        }
    }

    pub fn register_asset_type<AssetTypeFactoryT: AssetTypeHandlerFactory>(
        &mut self,
        asset_resource: &mut AssetResource,
    ) {
        let asset_type = AssetTypeFactoryT::create(asset_resource);
        let mut asset_registration_order = (*self.asset_registration_order).clone();
        asset_registration_order.push(asset_type.asset_type_id());
        self.asset_registration_order = Arc::new(asset_registration_order);
        let old = self
            .asset_types
            .insert(asset_type.asset_type_id(), asset_type);
        assert!(old.is_none());
    }

    pub fn register_default_asset_types(
        &mut self,
        asset_resource: &mut AssetResource,
    ) {
        self.register_asset_type::<ShaderAssetTypeHandler>(asset_resource);
        self.register_asset_type::<ComputePipelineAssetTypeHandler>(asset_resource);
        self.register_asset_type::<MaterialAssetTypeHandler>(asset_resource);
        self.register_asset_type::<MaterialInstanceAssetTypeHandler>(asset_resource);
        self.register_asset_type::<SamplerAssetTypeHandler>(asset_resource);
        self.register_asset_type::<ImageAssetTypeHandler>(asset_resource);
        self.register_asset_type::<BufferAssetTypeHandler>(asset_resource);
    }

    pub fn committed_asset<AssetT: 'static>(
        &self,
        handle: &Handle<AssetT>,
    ) -> Option<&AssetT> {
        let asset_type = self.asset_types.get(&TypeId::of::<AssetT>())?;
        asset_type
            .asset_lookup()
            .downcast_ref::<AssetLookup<AssetT>>()
            .unwrap()
            .get_committed(handle.load_handle())
    }

    pub fn latest_asset<AssetT: 'static>(
        &self,
        handle: &Handle<AssetT>,
    ) -> Option<&AssetT> {
        let asset_type = self.asset_types.get(&TypeId::of::<AssetT>())?;
        asset_type
            .asset_lookup()
            .downcast_ref::<AssetLookup<AssetT>>()
            .unwrap()
            .get_latest(handle.load_handle())
    }

    pub fn wait_for_asset_to_load<T>(
        &mut self,
        asset_handle: &distill::loader::handle::Handle<T>,
        asset_resource: &mut AssetResource,
        asset_name: &str,
    ) -> RafxResult<()> {
        const PRINT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(1000);
        let mut last_print_time = None;

        fn on_interval<F: Fn()>(
            interval: std::time::Duration,
            last_time: &mut Option<std::time::Instant>,
            f: F,
        ) {
            let now = std::time::Instant::now();

            if last_time.is_none() || now - last_time.unwrap() >= interval {
                (f)();
                *last_time = Some(now);
            }
        }

        loop {
            asset_resource.update();
            self.update_asset_loaders()?;
            match asset_resource.load_status(&asset_handle) {
                LoadStatus::NotRequested => {
                    unreachable!();
                }
                LoadStatus::Unresolved => {
                    on_interval(PRINT_INTERVAL, &mut last_print_time, || {
                        log::info!(
                            "blocked waiting for asset to resolve {} {:?}",
                            asset_name,
                            asset_handle
                        );
                    });
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                LoadStatus::Loading => {
                    on_interval(PRINT_INTERVAL, &mut last_print_time, || {
                        log::info!(
                            "blocked waiting for asset to load {} {:?}",
                            asset_name,
                            asset_handle
                        );
                    });
                    std::thread::sleep(std::time::Duration::from_millis(1));
                    // keep waiting
                }
                LoadStatus::Loaded => {
                    break Ok(());
                }
                LoadStatus::Unloading => unreachable!(),
                LoadStatus::DoesNotExist => {
                    println!("Essential asset not found");
                }
                LoadStatus::Error(err) => {
                    println!("Error loading essential asset {:?}", err);
                }
            }
        }
    }

    pub fn device_context(&self) -> &RafxDeviceContext {
        &self.device_context
    }

    pub fn graphics_queue(&self) -> &RafxQueue {
        &self.graphics_queue
    }

    pub fn transfer_queue(&self) -> &RafxQueue {
        &self.transfer_queue
    }

    pub fn resource_manager(&self) -> &ResourceManager {
        &self.resource_manager
    }

    pub fn resource_manager_mut(&mut self) -> &mut ResourceManager {
        &mut self.resource_manager
    }

    pub fn resources(&self) -> &ResourceLookupSet {
        self.resource_manager.resources()
    }

    pub fn graphics_pipeline_cache(&self) -> &GraphicsPipelineCache {
        self.resource_manager.graphics_pipeline_cache()
    }

    pub fn dyn_command_pool_allocator(&self) -> &DynCommandPoolAllocator {
        self.resource_manager.dyn_command_pool_allocator()
    }

    pub fn create_dyn_resource_allocator_set(&self) -> DynResourceAllocatorSet {
        self.resource_manager.create_dyn_resource_allocator_set()
    }

    pub fn create_dyn_resource_allocator_provider(&self) -> DynResourceAllocatorSetProvider {
        self.resource_manager
            .create_dyn_resource_allocator_provider()
    }

    pub fn create_descriptor_set_allocator(&self) -> DescriptorSetAllocatorRef {
        self.resource_manager.create_descriptor_set_allocator()
    }

    pub fn create_descriptor_set_allocator_provider(&self) -> DescriptorSetAllocatorProvider {
        self.resource_manager
            .create_descriptor_set_allocator_provider()
    }

    pub(crate) fn material_instance_descriptor_sets_mut(&mut self) -> &mut DescriptorSetAllocator {
        &mut self.material_instance_descriptor_sets
    }

    pub(crate) fn upload_manager(&self) -> &UploadManager {
        &self.upload_manager
    }

    //
    // Loaders
    //

    pub fn get_material_pass_by_index(
        &self,
        handle: &Handle<MaterialAsset>,
        index: usize,
    ) -> Option<ResourceArc<MaterialPassResource>> {
        self.committed_asset(handle)
            .and_then(|x| x.passes.get(index))
            .map(|x| x.material_pass_resource.clone())
    }

    pub fn get_descriptor_set_layout_for_pass(
        &self,
        handle: &Handle<MaterialAsset>,
        pass_index: usize,
        layout_index: usize,
    ) -> Option<ResourceArc<DescriptorSetLayoutResource>> {
        self.committed_asset(&handle)
            .and_then(|x| x.passes.get(pass_index))
            .map(|x| {
                x.material_pass_resource.get_raw().descriptor_set_layouts[layout_index].clone()
            })
    }

    // Call whenever you want to handle assets loading/unloading
    #[profiling::function]
    pub fn update_asset_loaders(&mut self) -> RafxResult<()> {
        for asset_type in &*self.asset_registration_order.clone() {
            let mut asset_type = self.asset_types.remove(asset_type).unwrap();
            asset_type.process_load_requests(self)?;
            self.asset_types
                .insert(asset_type.asset_type_id(), asset_type);
        }

        self.upload_manager.update()?;

        Ok(())
    }

    // Call just before rendering
    pub fn on_begin_frame(&mut self) -> RafxResult<()> {
        self.material_instance_descriptor_sets.flush_changes()
    }

    #[profiling::function]
    pub fn on_frame_complete(&mut self) -> RafxResult<()> {
        self.resource_manager.on_frame_complete()?;
        self.material_instance_descriptor_sets.on_frame_complete();
        Ok(())
    }

    pub fn metrics(&self) -> AssetManagerMetrics {
        //let loaded_asset_metrics = self.loaded_assets.metrics();
        let resource_manager_metrics = self.resource_manager.metrics();
        let material_instance_descriptor_sets_metrics =
            self.material_instance_descriptor_sets.metrics();

        AssetManagerMetrics {
            resource_manager_metrics,
            //loaded_asset_metrics,
            material_instance_descriptor_sets_metrics,
        }
    }

    #[profiling::function]
    pub fn apply_material_instance_slot_assignment(
        &self,
        slot_assignment: &MaterialInstanceSlotAssignment,
        pass_slot_name_lookup: &SlotNameLookup,
        resources: &ResourceLookupSet,
        material_pass_write_set: &mut Vec<DescriptorSetWriteSet>,
    ) -> RafxResult<()> {
        if let Some(slot_locations) = pass_slot_name_lookup.get(&slot_assignment.slot_name) {
            for location in slot_locations {
                log::trace!(
                    "Apply write to location {:?} via slot {}",
                    location,
                    slot_assignment.slot_name
                );
                let layout_descriptor_set_writes =
                    &mut material_pass_write_set[location.layout_index as usize];
                let write = layout_descriptor_set_writes
                    .elements
                    .get_mut(&DescriptorSetElementKey {
                        dst_binding: location.binding_index,
                    })
                    .unwrap();

                let what_to_bind = rafx_framework::descriptor_sets::what_to_bind(write);

                if what_to_bind.bind_images || what_to_bind.bind_samplers {
                    let mut write_image = DescriptorSetWriteElementImage {
                        image_view: None,
                        sampler: None,
                    };

                    if what_to_bind.bind_images {
                        if let Some(image) = &slot_assignment.image {
                            let loaded_image = self.latest_asset(&image).unwrap();
                            write_image.image_view =
                                Some(rafx_framework::descriptor_sets::DescriptorSetWriteElementImageValue::Resource(
                                    loaded_image.image_view.clone(),
                                ));
                        }
                    }

                    if what_to_bind.bind_samplers {
                        if let Some(sampler) = &slot_assignment.sampler {
                            let sampler = resources.get_or_create_sampler(sampler)?;
                            write_image.sampler = Some(sampler);
                        }
                    }

                    write.image_info = vec![write_image];
                }

                if what_to_bind.bind_buffers {
                    let mut write_buffer = DescriptorSetWriteElementBuffer { buffer: None };

                    if let Some(buffer_data) = &slot_assignment.buffer_data {
                        write_buffer.buffer = Some(DescriptorSetWriteElementBufferData::Data(
                            buffer_data.clone(),
                        ));
                    }

                    write.buffer_info = vec![write_buffer];
                }
            }
        }

        Ok(())
    }

    pub fn create_write_sets_for_material_instance_pass(
        &self,
        pass: &MaterialPass,
        slots: &[MaterialInstanceSlotAssignment],
        resources: &ResourceLookupSet,
    ) -> RafxResult<Vec<DescriptorSetWriteSet>> {
        let mut pass_descriptor_set_writes =
            pass.create_uninitialized_write_sets_for_material_pass();

        //
        // Now modify the descriptor set writes to actually point at the things specified by the material
        //
        for slot in slots {
            self.apply_material_instance_slot_assignment(
                slot,
                &pass.pass_slot_name_lookup,
                resources,
                &mut pass_descriptor_set_writes,
            )?;
        }

        Ok(pass_descriptor_set_writes)
    }
}

impl Drop for AssetManager {
    fn drop(&mut self) {
        log::info!("Cleaning up asset manager");
        log::trace!("Asset Manager Metrics:\n{:#?}", self.metrics());

        // Wait for queues to be idle before destroying resources
        self.transfer_queue.wait_for_queue_idle().unwrap();
        self.graphics_queue.wait_for_queue_idle().unwrap();

        // Clear in reverse order of registration. This way if asset type A holds a reference to
        // asset type B, A can be removed first, then B.
        for asset_type in self.asset_registration_order.iter().rev() {
            self.asset_types.remove(asset_type).unwrap();
        }

        // Drop all descriptors. These bind to raw resources, so we need to drop them before
        // dropping resources
        self.material_instance_descriptor_sets.destroy().unwrap();

        log::info!("Dropping asset manager");
        log::trace!("Asset Manager Metrics:\n{:#?}", self.metrics());
    }
}
