use crate::assets::ImageAssetData;
use crate::assets::ShaderAssetData;
use crate::assets::{
    BufferAsset, GraphicsPipelineAsset, ImageAsset, MaterialAsset, MaterialInstanceAsset,
    MaterialPass, RenderpassAsset, SamplerAsset, ShaderAsset,
};
use crate::assets::{
    GraphicsPipelineAssetData, MaterialAssetData, MaterialInstanceAssetData, RenderpassAssetData,
};
use crate::{
    AssetLookup, AssetLookupSet, BufferAssetData, ComputePipelineAsset, ComputePipelineAssetData,
    GenericLoader, LoadQueues, MaterialInstanceSlotAssignment, SamplerAssetData, SlotNameLookup,
    UploadQueueConfig,
};
use ash::prelude::*;
use atelier_assets::loader::handle::Handle;
use rafx_resources::{
    vk_description as dsc, ComputePipelineResource, DescriptorSetAllocatorMetrics,
    DescriptorSetAllocatorProvider, DescriptorSetAllocatorRef, DescriptorSetLayoutResource,
    DescriptorSetWriteSet, DynResourceAllocatorSet, GraphicsPipelineCache, MaterialPassResource,
    ResourceArc,
};
use rafx_shell_vulkan::{VkBuffer, VkDeviceContext, VkImage};

use super::asset_lookup::LoadedAssetMetrics;
use super::load_queue::LoadQueueSet;
use super::upload::{BufferUploadOpResult, ImageUploadOpResult, UploadManager};
use ash::vk;
use atelier_assets::loader::handle::AssetHandle;
use atelier_assets::loader::storage::AssetLoadOp;
use atelier_assets::loader::Loader;
use crossbeam_channel::Sender;
use fnv::FnvHashMap;
use rafx_nodes::RenderRegistry;
use rafx_resources::descriptor_sets::{
    DescriptorSetElementKey, DescriptorSetWriteElementBuffer, DescriptorSetWriteElementBufferData,
    DescriptorSetWriteElementImage,
};
use rafx_resources::vk_description::ShaderModuleMeta;
use rafx_resources::DescriptorSetAllocator;
use rafx_resources::DynCommandWriterAllocator;
use rafx_resources::DynResourceAllocatorSetProvider;
use rafx_resources::ResourceLookupSet;
use rafx_resources::{ResourceManager, ResourceManagerMetrics};
use std::sync::Arc;

#[derive(Debug)]
pub struct AssetManagerMetrics {
    pub resource_manager_metrics: ResourceManagerMetrics,
    pub loaded_asset_metrics: LoadedAssetMetrics,
    pub material_instance_descriptor_sets_metrics: DescriptorSetAllocatorMetrics,
}

pub struct AssetManagerLoaders {
    pub shader_loader: GenericLoader<ShaderAssetData, ShaderAsset>,
    pub graphics_pipeline_loader: GenericLoader<GraphicsPipelineAssetData, GraphicsPipelineAsset>,
    pub compute_pipeline_loader: GenericLoader<ComputePipelineAssetData, ComputePipelineAsset>,
    pub renderpass_loader: GenericLoader<RenderpassAssetData, RenderpassAsset>,
    pub material_loader: GenericLoader<MaterialAssetData, MaterialAsset>,
    pub material_instance_loader: GenericLoader<MaterialInstanceAssetData, MaterialInstanceAsset>,
    pub sampler_loader: GenericLoader<SamplerAssetData, SamplerAsset>,
    pub image_loader: GenericLoader<ImageAssetData, ImageAsset>,
    pub buffer_loader: GenericLoader<BufferAssetData, BufferAsset>,
}

pub struct AssetManager {
    resource_manager: ResourceManager,
    loaded_assets: AssetLookupSet,
    load_queues: LoadQueueSet,
    upload_manager: UploadManager,
    material_instance_descriptor_sets: DescriptorSetAllocator,
}

impl AssetManager {
    pub fn new(
        device_context: &VkDeviceContext,
        render_registry: &RenderRegistry,
        loader: &Loader,
        upload_queue_config: UploadQueueConfig,
    ) -> Self {
        let resource_manager = ResourceManager::new(device_context, render_registry);

        AssetManager {
            resource_manager,
            loaded_assets: AssetLookupSet::new(loader),
            load_queues: Default::default(),
            upload_manager: UploadManager::new(device_context, upload_queue_config),
            material_instance_descriptor_sets: DescriptorSetAllocator::new(device_context),
        }
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

    pub fn dyn_command_writer_allocator(&self) -> &DynCommandWriterAllocator {
        self.resource_manager.dyn_command_writer_allocator()
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

    fn create_graphics_pipeline_loader(
        &self
    ) -> GenericLoader<GraphicsPipelineAssetData, GraphicsPipelineAsset> {
        self.load_queues.graphics_pipelines.create_loader()
    }

    fn create_compute_pipeline_loader(
        &self
    ) -> GenericLoader<ComputePipelineAssetData, ComputePipelineAsset> {
        self.load_queues.compute_pipelines.create_loader()
    }

    fn create_renderpass_loader(&self) -> GenericLoader<RenderpassAssetData, RenderpassAsset> {
        self.load_queues.renderpasses.create_loader()
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
            graphics_pipeline_loader: self.create_graphics_pipeline_loader(),
            compute_pipeline_loader: self.create_compute_pipeline_loader(),
            renderpass_loader: self.create_renderpass_loader(),
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
            .map(|x| x.descriptor_set_layouts[layout_index].clone())
    }

    // Call whenever you want to handle assets loading/unloading
    #[profiling::function]
    pub fn update_asset_loaders(&mut self) -> VkResult<()> {
        self.process_shader_load_requests();
        self.process_graphics_pipeline_load_requests();
        self.process_compute_pipeline_load_requests();
        self.process_renderpass_load_requests();
        self.process_material_load_requests();
        self.process_material_instance_load_requests();
        self.process_sampler_load_requests();
        self.process_image_load_requests()?;
        self.process_buffer_load_requests()?;

        self.upload_manager.update()?;

        Ok(())
    }

    // Call just before rendering
    pub fn on_begin_frame(&mut self) -> VkResult<()> {
        self.material_instance_descriptor_sets.flush_changes()
    }

    #[profiling::function]
    pub fn on_frame_complete(&mut self) -> VkResult<()> {
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
            let loaded_asset = self.load_shader_module(&request.asset);
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
    fn process_graphics_pipeline_load_requests(&mut self) {
        for request in self.load_queues.graphics_pipelines.take_load_requests() {
            log::trace!("Create graphics pipeline {:?}", request.load_handle);
            let loaded_asset = self.load_graphics_pipeline(request.asset);
            Self::handle_load_result(
                request.load_op,
                loaded_asset,
                &mut self.loaded_assets.graphics_pipelines,
                request.result_tx,
            );
        }

        Self::handle_commit_requests(
            &mut self.load_queues.graphics_pipelines,
            &mut self.loaded_assets.graphics_pipelines,
        );
        Self::handle_free_requests(
            &mut self.load_queues.graphics_pipelines,
            &mut self.loaded_assets.graphics_pipelines,
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
    fn process_renderpass_load_requests(&mut self) {
        for request in self.load_queues.renderpasses.take_load_requests() {
            log::trace!("Create renderpass {:?}", request.load_handle);
            let loaded_asset = self.load_renderpass(request.asset);
            Self::handle_load_result(
                request.load_op,
                loaded_asset,
                &mut self.loaded_assets.renderpasses,
                request.result_tx,
            );
        }

        Self::handle_commit_requests(
            &mut self.load_queues.renderpasses,
            &mut self.loaded_assets.renderpasses,
        );
        Self::handle_free_requests(
            &mut self.load_queues.renderpasses,
            &mut self.loaded_assets.renderpasses,
        );
    }

    #[profiling::function]
    fn process_material_load_requests(&mut self) {
        for request in self.load_queues.materials.take_load_requests() {
            log::trace!("Create material {:?}", request.load_handle);
            let loaded_asset = self.load_material(&request.asset);
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
            let loaded_asset = self.load_material_instance(&request.asset);
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
            let loaded_asset = self.load_sampler(&request.asset);
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
    fn process_image_load_requests(&mut self) -> VkResult<()> {
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
                ImageUploadOpResult::UploadComplete(load_op, result_tx, image) => {
                    log::trace!("Uploading image {:?} complete", load_op.load_handle());
                    let loaded_asset = self.finish_load_image(image);
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
    fn process_buffer_load_requests(&mut self) -> VkResult<()> {
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
        loaded_asset: VkResult<AssetT>,
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
        image: VkImage,
    ) -> VkResult<ImageAsset> {
        let format = image.format.into();
        let mip_level_count = image.mip_level_count;

        let image = self.resources().insert_image(image);

        let image_view_meta = dsc::ImageViewMeta {
            view_type: dsc::ImageViewType::Type2D,
            format,
            subresource_range: dsc::ImageSubresourceRange {
                aspect_mask: dsc::ImageAspectFlag::Color.into(),
                base_mip_level: 0,
                level_count: mip_level_count,
                base_array_layer: 0,
                layer_count: 1,
            },
            components: dsc::ComponentMapping {
                r: dsc::ComponentSwizzle::Identity,
                g: dsc::ComponentSwizzle::Identity,
                b: dsc::ComponentSwizzle::Identity,
                a: dsc::ComponentSwizzle::Identity,
            },
        };

        let image_view = self
            .resources()
            .get_or_create_image_view(&image, &image_view_meta)?;

        Ok(ImageAsset { image_view })
    }

    #[profiling::function]
    fn finish_load_buffer(
        &mut self,
        buffer: VkBuffer,
    ) -> VkResult<BufferAsset> {
        let buffer = self.resources().insert_buffer(buffer);

        Ok(BufferAsset { buffer })
    }

    #[profiling::function]
    fn load_shader_module(
        &mut self,
        shader_module: &ShaderAssetData,
    ) -> VkResult<ShaderAsset> {
        let shader = Arc::new(shader_module.shader.clone());

        let mut reflection_data_lookup = FnvHashMap::default();
        if let Some(reflection_data) = &shader_module.reflection_data {
            for entry_point in reflection_data {
                let old =
                    reflection_data_lookup.insert(entry_point.name.clone(), entry_point.clone());
                assert!(old.is_none());
            }
        }

        let shader_module = self.resources().get_or_create_shader_module(&shader)?;
        Ok(ShaderAsset {
            shader_module,
            reflection_data: Arc::new(reflection_data_lookup),
        })
    }

    #[profiling::function]
    fn load_sampler(
        &mut self,
        sampler: &SamplerAssetData,
    ) -> VkResult<SamplerAsset> {
        let sampler = self.resources().get_or_create_sampler(&sampler.sampler)?;
        Ok(SamplerAsset { sampler })
    }

    #[profiling::function]
    fn load_graphics_pipeline(
        &mut self,
        graphics_pipeline_asset_data: GraphicsPipelineAssetData,
    ) -> VkResult<GraphicsPipelineAsset> {
        Ok(GraphicsPipelineAsset {
            pipeline_asset: Arc::new(graphics_pipeline_asset_data),
        })
    }

    #[profiling::function]
    fn load_compute_pipeline(
        &mut self,
        compute_pipeline_asset_data: ComputePipelineAssetData,
    ) -> VkResult<ComputePipelineAsset> {
        //
        // Get the shader module
        //
        let shader_module = self
            .assets()
            .shader_modules
            .get_latest(compute_pipeline_asset_data.shader_module.load_handle())
            .unwrap();
        let shader_module_meta = ShaderModuleMeta {
            entry_name: compute_pipeline_asset_data.entry_name,
            stage: dsc::ShaderStage::Compute,
        };

        //
        // Find the reflection data in the shader module for the given entry point
        //
        let reflection_data = shader_module
            .reflection_data
            .get(&shader_module_meta.entry_name);
        let reflection_data = reflection_data.ok_or_else(|| {
            log::error!(
                "Load Compute Shader Failed - Pass refers to entry point named {}, but no matching reflection data was found",
                shader_module_meta.entry_name
            );
            vk::Result::ERROR_UNKNOWN
        })?;

        //
        // Create the push constant ranges
        //
        let mut push_constant_ranges = vec![];
        for (range_index, range) in reflection_data.push_constants.iter().enumerate() {
            log::trace!("    Add range index {} {:?}", range_index, range);
            push_constant_ranges.push(range.push_constant.clone());
        }

        //
        // Gather the descriptor set bindings
        //
        let mut descriptor_set_layout_defs = Vec::default();
        for (set_index, layout) in reflection_data.descriptor_set_layouts.iter().enumerate() {
            // Expand the layout def to include the given set index
            while descriptor_set_layout_defs.len() <= set_index {
                descriptor_set_layout_defs.push(dsc::DescriptorSetLayout::default());
            }

            if let Some(layout) = layout.as_ref() {
                for binding in &layout.bindings {
                    let def = dsc::DescriptorSetLayoutBinding {
                        binding: binding.binding,
                        descriptor_type: binding.descriptor_type,
                        descriptor_count: binding.descriptor_count,
                        stage_flags: binding.stage_flags,
                        immutable_samplers: binding.immutable_samplers.clone(),
                        internal_buffer_per_descriptor_size: binding
                            .internal_buffer_per_descriptor_size,
                    };

                    log::trace!(
                        "    Add descriptor binding set={} binding={} for stage {:?}",
                        set_index,
                        binding.binding,
                        binding.stage_flags
                    );

                    descriptor_set_layout_defs[set_index]
                        .descriptor_set_layout_bindings
                        .push(def);
                }
            }
        }

        //
        // Create the descriptor set layout
        //
        let mut descriptor_set_layouts = Vec::with_capacity(descriptor_set_layout_defs.len());

        for descriptor_set_layout_def in &descriptor_set_layout_defs {
            let descriptor_set_layout = self
                .resources()
                .get_or_create_descriptor_set_layout(&descriptor_set_layout_def)?;
            descriptor_set_layouts.push(descriptor_set_layout);
        }

        //
        // Create the pipeline layout
        //
        let pipeline_layout_def = dsc::PipelineLayout {
            descriptor_set_layouts: descriptor_set_layout_defs,
            push_constant_ranges,
        };

        let pipeline_layout = self
            .resources()
            .get_or_create_pipeline_layout(&pipeline_layout_def)?;

        //
        // Create the compute pipeline
        //
        let compute_pipeline = self.resources().get_or_create_compute_pipeline(
            shader_module.shader_module.clone(),
            shader_module_meta,
            pipeline_layout,
        )?;

        Ok(ComputePipelineAsset { compute_pipeline })
    }

    #[profiling::function]
    fn load_renderpass(
        &mut self,
        renderpass_asset: RenderpassAssetData,
    ) -> VkResult<RenderpassAsset> {
        Ok(RenderpassAsset {
            renderpass_def: Arc::new(renderpass_asset.renderpass),
        })
    }

    #[profiling::function]
    fn load_material(
        &mut self,
        material_asset: &MaterialAssetData,
    ) -> VkResult<MaterialAsset> {
        let mut passes = Vec::with_capacity(material_asset.passes.len());
        let mut pass_name_to_index = FnvHashMap::default();
        let mut pass_phase_to_index = FnvHashMap::default();

        for pass_data in &material_asset.passes {
            let pass = MaterialPass::new(self, pass_data)?;

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
                    log::error!(
                            "Load Material Failed - Pass refers to phase name {}, but this phase name was not registered",
                            phase_name
                        );
                    return Err(vk::Result::ERROR_UNKNOWN);
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
        material_instance_asset: &MaterialInstanceAssetData,
    ) -> VkResult<MaterialInstanceAsset> {
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

            let material_pass_descriptor_set_layouts = &pass
                .material_pass_resource
                .get_raw()
                .pipeline_layout
                .get_raw()
                .descriptor_sets;

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
            material_instance_asset.material.clone(),
            material_asset.clone(),
            material_descriptor_sets,
            material_instance_asset.slot_assignments.clone(),
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
    ) -> VkResult<()> {
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

                let what_to_bind = rafx_resources::descriptor_sets::what_to_bind(write);

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
                                Some(rafx_resources::descriptor_sets::DescriptorSetWriteElementImageValue::Resource(
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
    ) -> VkResult<Vec<DescriptorSetWriteSet>> {
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

        // Wipe out any loaded assets. This will potentially drop ref counts on resources
        self.loaded_assets.destroy();

        // Drop all descriptors. These bind to raw resources, so we need to drop them before
        // dropping resources
        self.material_instance_descriptor_sets.destroy().unwrap();

        log::info!("Dropping asset manager");
        log::trace!("Asset Manager Metrics:\n{:#?}", self.metrics());
    }
}
