use crate::assets::ImageAssetData;
use crate::assets::ShaderAssetData;
use crate::assets::{
    BufferAsset, ImageAsset, MaterialAsset, MaterialInstanceAsset, SamplerAsset, ShaderAsset,
};
use crate::assets::{MaterialAssetData, MaterialInstanceAssetData};
use crate::{
    AssetLookup, AssetLookupSet, BufferAssetData, ComputePipelineAsset, ComputePipelineAssetData,
    GenericLoader, LoadQueues, MaterialInstanceSlotAssignment, SamplerAssetData, UploadQueueConfig,
};
use distill::loader::handle::Handle;
use rafx_framework::{
    ComputePipelineResource, DescriptorSetAllocatorMetrics, DescriptorSetAllocatorProvider,
    DescriptorSetAllocatorRef, DescriptorSetLayout, DescriptorSetLayoutResource,
    DescriptorSetWriteSet, DynResourceAllocatorSet, GraphicsPipelineCache, MaterialPass,
    MaterialPassResource, ResourceArc, SlotNameLookup,
};

use super::asset_lookup::LoadedAssetMetrics;
use super::load_queue::LoadQueueSet;
use super::upload::{BufferUploadOpResult, ImageUploadOpResult, UploadManager};
use crossbeam_channel::Sender;
use distill::loader::handle::AssetHandle;
use distill::loader::storage::AssetLoadOp;
use distill::loader::Loader;
use fnv::FnvHashMap;
use rafx_api::{RafxBuffer, RafxDeviceContext, RafxQueue, RafxResult, RafxTexture};
use rafx_framework::descriptor_sets::{
    DescriptorSetElementKey, DescriptorSetWriteElementBuffer, DescriptorSetWriteElementBufferData,
    DescriptorSetWriteElementImage,
};
use rafx_framework::DescriptorSetAllocator;
use rafx_framework::DynCommandPoolAllocator;
use rafx_framework::DynResourceAllocatorSetProvider;
use rafx_framework::ResourceLookupSet;
use rafx_framework::{ResourceManager, ResourceManagerMetrics};
use rafx_framework::nodes::RenderRegistry;
use std::sync::Arc;

#[derive(Debug)]
pub struct AssetManagerMetrics {
    pub resource_manager_metrics: ResourceManagerMetrics,
    pub loaded_asset_metrics: LoadedAssetMetrics,
    pub material_instance_descriptor_sets_metrics: DescriptorSetAllocatorMetrics,
}

pub struct AssetManagerLoaders {
    pub shader_loader: GenericLoader<ShaderAssetData, ShaderAsset>,
    pub compute_pipeline_loader: GenericLoader<ComputePipelineAssetData, ComputePipelineAsset>,
    pub material_loader: GenericLoader<MaterialAssetData, MaterialAsset>,
    pub material_instance_loader: GenericLoader<MaterialInstanceAssetData, MaterialInstanceAsset>,
    pub sampler_loader: GenericLoader<SamplerAssetData, SamplerAsset>,
    pub image_loader: GenericLoader<ImageAssetData, ImageAsset>,
    pub buffer_loader: GenericLoader<BufferAssetData, BufferAsset>,
}

pub struct AssetManager {
    device_context: RafxDeviceContext,
    resource_manager: ResourceManager,
    loaded_assets: AssetLookupSet,
    load_queues: LoadQueueSet,
    upload_manager: UploadManager,
    material_instance_descriptor_sets: DescriptorSetAllocator,
    graphics_queue: RafxQueue,
    transfer_queue: RafxQueue,
}

impl AssetManager {
    pub fn new(
        device_context: &RafxDeviceContext,
        render_registry: &RenderRegistry,
        loader: &Loader,
        upload_queue_config: UploadQueueConfig,
        graphics_queue: &RafxQueue,
        transfer_queue: &RafxQueue,
    ) -> Self {
        let resource_manager = ResourceManager::new(device_context, render_registry);

        AssetManager {
            device_context: device_context.clone(),
            resource_manager,
            loaded_assets: AssetLookupSet::new(loader),
            load_queues: Default::default(),
            upload_manager: UploadManager::new(
                device_context,
                upload_queue_config,
                graphics_queue.clone(),
                transfer_queue.clone(),
            ),
            material_instance_descriptor_sets: DescriptorSetAllocator::new(device_context),
            graphics_queue: graphics_queue.clone(),
            transfer_queue: transfer_queue.clone(),
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

    //
    // Asset-specific accessors
    //
    #[allow(dead_code)]
    pub(super) fn assets(&self) -> &AssetLookupSet {
        &self.loaded_assets
    }

    #[allow(dead_code)]
    pub(super) fn assets_mut(&mut self) -> &mut AssetLookupSet {
        &mut self.loaded_assets
    }

    pub fn loaded_assets(&self) -> &AssetLookupSet {
        &self.loaded_assets
    }

    //
    // Loaders
    //
    fn create_shader_loader(&self) -> GenericLoader<ShaderAssetData, ShaderAsset> {
        self.load_queues.shader_modules.create_loader()
    }

    fn create_compute_pipeline_loader(
        &self
    ) -> GenericLoader<ComputePipelineAssetData, ComputePipelineAsset> {
        self.load_queues.compute_pipelines.create_loader()
    }

    fn create_material_loader(&self) -> GenericLoader<MaterialAssetData, MaterialAsset> {
        self.load_queues.materials.create_loader()
    }

    fn create_material_instance_loader(
        &self
    ) -> GenericLoader<MaterialInstanceAssetData, MaterialInstanceAsset> {
        self.load_queues.material_instances.create_loader()
    }

    fn create_sampler_loader(&self) -> GenericLoader<SamplerAssetData, SamplerAsset> {
        self.load_queues.samplers.create_loader()
    }

    fn create_image_loader(&self) -> GenericLoader<ImageAssetData, ImageAsset> {
        self.load_queues.images.create_loader()
    }

    fn create_buffer_loader(&self) -> GenericLoader<BufferAssetData, BufferAsset> {
        self.load_queues.buffers.create_loader()
    }

    pub fn create_loaders(&self) -> AssetManagerLoaders {
        AssetManagerLoaders {
            shader_loader: self.create_shader_loader(),
            compute_pipeline_loader: self.create_compute_pipeline_loader(),
            material_loader: self.create_material_loader(),
            material_instance_loader: self.create_material_instance_loader(),
            sampler_loader: self.create_sampler_loader(),
            image_loader: self.create_image_loader(),
            buffer_loader: self.create_buffer_loader(),
        }
    }

    //
    // Find things by asset handle
    //
    pub fn get_image_asset(
        &self,
        handle: &Handle<ImageAsset>,
    ) -> Option<&ImageAsset> {
        self.loaded_assets
            .images
            .get_committed(handle.load_handle())
    }

    pub fn get_material_pass_by_index(
        &self,
        handle: &Handle<MaterialAsset>,
        index: usize,
    ) -> Option<ResourceArc<MaterialPassResource>> {
        self.loaded_assets
            .materials
            .get_committed(handle.load_handle())
            .and_then(|x| x.passes.get(index))
            .map(|x| x.material_pass_resource.clone())
    }

    pub fn get_compute_pipeline(
        &self,
        handle: &Handle<ComputePipelineAsset>,
    ) -> Option<ResourceArc<ComputePipelineResource>> {
        self.loaded_assets
            .compute_pipelines
            .get_committed(handle.load_handle())
            .and_then(|x| Some(x.compute_pipeline.clone()))
    }

    pub fn get_descriptor_set_layout_for_pass(
        &self,
        handle: &Handle<MaterialAsset>,
        pass_index: usize,
        layout_index: usize,
    ) -> Option<ResourceArc<DescriptorSetLayoutResource>> {
        self.loaded_assets
            .materials
            .get_committed(handle.load_handle())
            .and_then(|x| x.passes.get(pass_index))
            .map(|x| {
                x.material_pass_resource.get_raw().descriptor_set_layouts[layout_index].clone()
            })
    }

    // Call whenever you want to handle assets loading/unloading
    #[profiling::function]
    pub fn update_asset_loaders(&mut self) -> RafxResult<()> {
        self.process_shader_load_requests();
        self.process_compute_pipeline_load_requests();
        self.process_material_load_requests();
        self.process_material_instance_load_requests();
        self.process_sampler_load_requests();
        self.process_image_load_requests()?;
        self.process_buffer_load_requests()?;

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
        let loaded_asset_metrics = self.loaded_assets.metrics();
        let resource_manager_metrics = self.resource_manager.metrics();
        let material_instance_descriptor_sets_metrics =
            self.material_instance_descriptor_sets.metrics();

        AssetManagerMetrics {
            resource_manager_metrics,
            loaded_asset_metrics,
            material_instance_descriptor_sets_metrics,
        }
    }

    #[profiling::function]
    fn process_shader_load_requests(&mut self) {
        for request in self.load_queues.shader_modules.take_load_requests() {
            log::trace!("Create shader module {:?}", request.load_handle);
            let loaded_asset = self.load_shader_module(request.asset);
            Self::handle_load_result(
                request.load_op,
                loaded_asset,
                &mut self.loaded_assets.shader_modules,
                request.result_tx,
            );
        }

        Self::handle_commit_requests(
            &mut self.load_queues.shader_modules,
            &mut self.loaded_assets.shader_modules,
        );
        Self::handle_free_requests(
            &mut self.load_queues.shader_modules,
            &mut self.loaded_assets.shader_modules,
        );
    }

    #[profiling::function]
    fn process_compute_pipeline_load_requests(&mut self) {
        for request in self.load_queues.compute_pipelines.take_load_requests() {
            log::trace!("Create compute pipeline {:?}", request.load_handle);
            let loaded_asset = self.load_compute_pipeline(request.asset);
            Self::handle_load_result(
                request.load_op,
                loaded_asset,
                &mut self.loaded_assets.compute_pipelines,
                request.result_tx,
            );
        }

        Self::handle_commit_requests(
            &mut self.load_queues.compute_pipelines,
            &mut self.loaded_assets.compute_pipelines,
        );
        Self::handle_free_requests(
            &mut self.load_queues.compute_pipelines,
            &mut self.loaded_assets.compute_pipelines,
        );
    }

    #[profiling::function]
    fn process_material_load_requests(&mut self) {
        for request in self.load_queues.materials.take_load_requests() {
            log::trace!("Create material {:?}", request.load_handle);
            let loaded_asset = self.load_material(request.asset);
            Self::handle_load_result(
                request.load_op,
                loaded_asset,
                &mut self.loaded_assets.materials,
                request.result_tx,
            );
        }

        Self::handle_commit_requests(
            &mut self.load_queues.materials,
            &mut self.loaded_assets.materials,
        );
        Self::handle_free_requests(
            &mut self.load_queues.materials,
            &mut self.loaded_assets.materials,
        );
    }

    #[profiling::function]
    fn process_material_instance_load_requests(&mut self) {
        for request in self.load_queues.material_instances.take_load_requests() {
            log::trace!("Create material instance {:?}", request.load_handle);
            let loaded_asset = self.load_material_instance(request.asset);
            Self::handle_load_result(
                request.load_op,
                loaded_asset,
                &mut self.loaded_assets.material_instances,
                request.result_tx,
            );
        }

        Self::handle_commit_requests(
            &mut self.load_queues.material_instances,
            &mut self.loaded_assets.material_instances,
        );
        Self::handle_free_requests(
            &mut self.load_queues.material_instances,
            &mut self.loaded_assets.material_instances,
        );
    }

    #[profiling::function]
    fn process_sampler_load_requests(&mut self) {
        for request in self.load_queues.samplers.take_load_requests() {
            log::trace!("Create sampler {:?}", request.load_handle);
            let loaded_asset = self.load_sampler(request.asset);
            Self::handle_load_result(
                request.load_op,
                loaded_asset,
                &mut self.loaded_assets.samplers,
                request.result_tx,
            );
        }

        Self::handle_commit_requests(
            &mut self.load_queues.samplers,
            &mut self.loaded_assets.samplers,
        );
        Self::handle_free_requests(
            &mut self.load_queues.samplers,
            &mut self.loaded_assets.samplers,
        );
    }

    #[profiling::function]
    fn process_image_load_requests(&mut self) -> RafxResult<()> {
        for request in self.load_queues.images.take_load_requests() {
            //TODO: Route the request directly to the upload queue
            log::trace!("Uploading image {:?}", request.load_handle);
            self.upload_manager.upload_image(request)?;
        }

        let results: Vec<_> = self
            .upload_manager
            .image_upload_result_rx
            .try_iter()
            .collect();
        for result in results {
            match result {
                ImageUploadOpResult::UploadComplete(load_op, result_tx, texture) => {
                    log::trace!("Uploading image {:?} complete", load_op.load_handle());
                    let loaded_asset = self.finish_load_image(texture);
                    Self::handle_load_result(
                        load_op,
                        loaded_asset,
                        &mut self.loaded_assets.images,
                        result_tx,
                    );
                }
                ImageUploadOpResult::UploadError(load_handle) => {
                    log::trace!("Uploading image {:?} failed", load_handle);
                    // Don't need to do anything - the uploaded should have triggered an error on the load_op
                }
                ImageUploadOpResult::UploadDrop(load_handle) => {
                    log::trace!("Uploading image {:?} cancelled", load_handle);
                    // Don't need to do anything - the uploaded should have triggered an error on the load_op
                }
            }
        }

        Self::handle_commit_requests(&mut self.load_queues.images, &mut self.loaded_assets.images);
        Self::handle_free_requests(&mut self.load_queues.images, &mut self.loaded_assets.images);
        Ok(())
    }

    #[profiling::function]
    fn process_buffer_load_requests(&mut self) -> RafxResult<()> {
        for request in self.load_queues.buffers.take_load_requests() {
            //TODO: Route the request directly to the upload queue
            log::trace!("Uploading buffer {:?}", request.load_handle);
            self.upload_manager.upload_buffer(request)?;
        }

        let results: Vec<_> = self
            .upload_manager
            .buffer_upload_result_rx
            .try_iter()
            .collect();
        for result in results {
            match result {
                BufferUploadOpResult::UploadComplete(load_op, result_tx, buffer) => {
                    log::trace!("Uploading buffer {:?} complete", load_op.load_handle());
                    let loaded_asset = self.finish_load_buffer(buffer);
                    Self::handle_load_result(
                        load_op,
                        loaded_asset,
                        &mut self.loaded_assets.buffers,
                        result_tx,
                    );
                }
                BufferUploadOpResult::UploadError(load_handle) => {
                    log::trace!("Uploading buffer {:?} failed", load_handle);
                    // Don't need to do anything - the uploaded should have triggered an error on the load_op
                }
                BufferUploadOpResult::UploadDrop(load_handle) => {
                    log::trace!("Uploading buffer {:?} cancelled", load_handle);
                    // Don't need to do anything - the uploaded should have triggered an error on the load_op
                }
            }
        }

        Self::handle_commit_requests(
            &mut self.load_queues.buffers,
            &mut self.loaded_assets.buffers,
        );
        Self::handle_free_requests(
            &mut self.load_queues.buffers,
            &mut self.loaded_assets.buffers,
        );

        Ok(())
    }

    fn handle_load_result<AssetT: Clone>(
        load_op: AssetLoadOp,
        loaded_asset: RafxResult<AssetT>,
        asset_lookup: &mut AssetLookup<AssetT>,
        result_tx: Sender<AssetT>,
    ) {
        match loaded_asset {
            Ok(loaded_asset) => {
                asset_lookup.set_uncommitted(load_op.load_handle(), loaded_asset.clone());
                result_tx.send(loaded_asset).unwrap();
                load_op.complete()
            }
            Err(err) => {
                load_op.error(err);
            }
        }
    }

    fn handle_commit_requests<AssetDataT, AssetT>(
        load_queues: &mut LoadQueues<AssetDataT, AssetT>,
        asset_lookup: &mut AssetLookup<AssetT>,
    ) {
        for request in load_queues.take_commit_requests() {
            log::trace!(
                "commit asset {:?} {}",
                request.load_handle,
                core::any::type_name::<AssetDataT>()
            );
            asset_lookup.commit(request.load_handle);
        }
    }

    fn handle_free_requests<AssetDataT, AssetT>(
        load_queues: &mut LoadQueues<AssetDataT, AssetT>,
        asset_lookup: &mut AssetLookup<AssetT>,
    ) {
        for request in load_queues.take_commit_requests() {
            asset_lookup.commit(request.load_handle);
        }
    }

    #[profiling::function]
    fn finish_load_image(
        &mut self,
        texture: RafxTexture,
    ) -> RafxResult<ImageAsset> {
        let image = self.resources().insert_image(texture);

        let image_view = self.resources().get_or_create_image_view(&image, None)?;

        Ok(ImageAsset { image, image_view })
    }

    #[profiling::function]
    fn finish_load_buffer(
        &mut self,
        buffer: RafxBuffer,
    ) -> RafxResult<BufferAsset> {
        let buffer = self.resources().insert_buffer(buffer);

        Ok(BufferAsset { buffer })
    }

    #[profiling::function]
    fn load_shader_module(
        &mut self,
        shader_module: ShaderAssetData,
    ) -> RafxResult<ShaderAsset> {
        let mut reflection_data_lookup = FnvHashMap::default();
        if let Some(reflection_data) = &shader_module.reflection_data {
            for entry_point in reflection_data {
                let old = reflection_data_lookup.insert(
                    entry_point.rafx_api_reflection.entry_point_name.clone(),
                    entry_point.clone(),
                );
                assert!(old.is_none());
            }
        }

        let shader_module = self.resources().get_or_create_shader_module(
            &shader_module.shader_package,
            Some(shader_module.shader_module_hash),
        )?;

        Ok(ShaderAsset {
            shader_module,
            reflection_data: Arc::new(reflection_data_lookup),
        })
    }

    #[profiling::function]
    fn load_sampler(
        &mut self,
        sampler: SamplerAssetData,
    ) -> RafxResult<SamplerAsset> {
        let sampler = self.resources().get_or_create_sampler(&sampler.sampler)?;
        Ok(SamplerAsset { sampler })
    }

    #[profiling::function]
    fn load_compute_pipeline(
        &mut self,
        compute_pipeline_asset_data: ComputePipelineAssetData,
    ) -> RafxResult<ComputePipelineAsset> {
        //
        // Get the shader module
        //
        let shader_module = self
            .assets()
            .shader_modules
            .get_latest(compute_pipeline_asset_data.shader_module.load_handle())
            .unwrap();

        //
        // Find the reflection data in the shader module for the given entry point
        //
        let reflection_data = shader_module
            .reflection_data
            .get(&compute_pipeline_asset_data.entry_name);
        let reflection_data = reflection_data.ok_or_else(|| {
            let error_message = format!(
                "Load Compute Shader Failed - Pass refers to entry point named {}, but no matching reflection data was found",
                compute_pipeline_asset_data.entry_name
            );
            log::error!("{}", error_message);
            error_message
        })?;

        let shader = self
            .resources()
            .get_or_create_shader(&[shader_module.shader_module.clone()], &[&reflection_data])?;

        let root_signature =
            self.resources()
                .get_or_create_root_signature(&[shader.clone()], &[], &[])?;

        //
        // Create the push constant ranges
        //

        // Currently unused, can be handled by the rafx api layer
        // let mut push_constant_ranges = vec![];
        // for (range_index, range) in reflection_data.push_constants.iter().enumerate() {
        //     log::trace!("    Add range index {} {:?}", range_index, range);
        //     push_constant_ranges.push(range.push_constant.clone());
        // }

        //
        // Gather the descriptor set bindings
        //
        let mut descriptor_set_layout_defs = Vec::default();
        for (set_index, layout) in reflection_data.descriptor_set_layouts.iter().enumerate() {
            // Expand the layout def to include the given set index
            while descriptor_set_layout_defs.len() <= set_index {
                descriptor_set_layout_defs.push(DescriptorSetLayout::default());
            }

            if let Some(layout) = layout.as_ref() {
                for binding in &layout.bindings {
                    log::trace!(
                        "    Add descriptor binding set={} binding={} for stage {:?}",
                        set_index,
                        binding.resource.binding,
                        binding.resource.used_in_shader_stages
                    );
                    let def = binding.clone().into();

                    descriptor_set_layout_defs[set_index].bindings.push(def);
                }
            }
        }

        //
        // Create the descriptor set layout
        //
        let mut descriptor_set_layouts = Vec::with_capacity(descriptor_set_layout_defs.len());

        for (set_index, descriptor_set_layout_def) in descriptor_set_layout_defs.iter().enumerate()
        {
            let descriptor_set_layout = self.resources().get_or_create_descriptor_set_layout(
                &root_signature,
                set_index as u32,
                &descriptor_set_layout_def,
            )?;
            descriptor_set_layouts.push(descriptor_set_layout);
        }

        //
        // Create the compute pipeline
        //
        let compute_pipeline = self.resources().get_or_create_compute_pipeline(
            &shader,
            &root_signature,
            descriptor_set_layouts,
        )?;

        Ok(ComputePipelineAsset { compute_pipeline })
    }

    #[profiling::function]
    fn load_material(
        &mut self,
        material_asset: MaterialAssetData,
    ) -> RafxResult<MaterialAsset> {
        let mut passes = Vec::with_capacity(material_asset.passes.len());
        let mut pass_name_to_index = FnvHashMap::default();
        let mut pass_phase_to_index = FnvHashMap::default();

        for pass_data in &material_asset.passes {
            let pass = pass_data.create_material_pass(self)?;

            let pass_index = passes.len();
            passes.push(pass);

            if let Some(name) = &pass_data.name {
                let old = pass_name_to_index.insert(name.clone(), pass_index);
                assert!(old.is_none());
            }

            if let Some(phase_name) = &pass_data.phase {
                if let Some(phase_index) = self
                    .resource_manager
                    .render_registry()
                    .render_phase_index_from_name(phase_name)
                {
                    let old = pass_phase_to_index.insert(phase_index, pass_index);
                    assert!(old.is_none());
                } else {
                    let error = format!(
                        "Load Material Failed - Pass refers to phase name {}, but this phase name was not registered",
                        phase_name
                    );
                    log::error!("{}", error);
                    return Err(error)?;
                }
            }
        }

        Ok(MaterialAsset::new(
            passes,
            pass_name_to_index,
            pass_phase_to_index,
        ))
    }

    #[profiling::function]
    fn load_material_instance(
        &mut self,
        material_instance_asset: MaterialInstanceAssetData,
    ) -> RafxResult<MaterialInstanceAsset> {
        // Find the material we will bind over, we need the metadata from it
        let material_asset = self
            .loaded_assets
            .materials
            .get_latest(material_instance_asset.material.load_handle())
            .unwrap();

        let mut material_instance_descriptor_set_writes =
            Vec::with_capacity(material_asset.passes.len());

        log::trace!(
            "load_material_instance slot assignments\n{:#?}",
            material_instance_asset.slot_assignments
        );

        // This will be references to descriptor sets. Indexed by pass, and then by set within the pass.
        let mut material_descriptor_sets = Vec::with_capacity(material_asset.passes.len());
        for pass in &*material_asset.passes {
            let pass_descriptor_set_writes = self.create_write_sets_for_material_instance_pass(
                pass,
                &material_instance_asset.slot_assignments,
                self.resources(),
            )?;

            log::trace!(
                "load_material_instance descriptor set write\n{:#?}",
                pass_descriptor_set_writes
            );

            material_instance_descriptor_set_writes.push(pass_descriptor_set_writes.clone());

            // This will contain the descriptor sets created for this pass, one for each set within the pass
            let mut pass_descriptor_sets = Vec::with_capacity(pass_descriptor_set_writes.len());

            let material_pass_descriptor_set_layouts =
                &pass.material_pass_resource.get_raw().descriptor_set_layouts;

            //
            // Register the writes into the correct descriptor set pools
            //
            for (layout_index, layout_writes) in pass_descriptor_set_writes.into_iter().enumerate()
            {
                if !layout_writes.elements.is_empty() {
                    let descriptor_set = self
                        .material_instance_descriptor_sets
                        .create_descriptor_set_with_writes(
                            &material_pass_descriptor_set_layouts[layout_index],
                            layout_writes,
                        )?;

                    pass_descriptor_sets.push(Some(descriptor_set));
                } else {
                    // If there are no descriptors in this layout index, assume the layout does not
                    // exist
                    pass_descriptor_sets.push(None);
                }
            }

            material_descriptor_sets.push(pass_descriptor_sets);
        }

        log::trace!("Loaded material\n{:#?}", material_descriptor_sets);

        // Put these in an arc to avoid cloning the underlying data repeatedly
        let material_descriptor_sets = Arc::new(material_descriptor_sets);
        Ok(MaterialInstanceAsset::new(
            material_instance_asset.material,
            material_asset.clone(),
            material_descriptor_sets,
            material_instance_asset.slot_assignments,
            material_instance_descriptor_set_writes,
        ))
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
                            let loaded_image = self
                                .loaded_assets
                                .images
                                .get_latest(image.load_handle())
                                .unwrap();
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

        // Wipe out any loaded assets. This will potentially drop ref counts on resources
        self.loaded_assets.destroy();

        // Drop all descriptors. These bind to raw resources, so we need to drop them before
        // dropping resources
        self.material_instance_descriptor_sets.destroy().unwrap();

        log::info!("Dropping asset manager");
        log::trace!("Asset Manager Metrics:\n{:#?}", self.metrics());
    }
}
