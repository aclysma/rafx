use renderer_shell_vulkan::{VkDeviceContext, VkImage, VkImageRaw, VkBuffer};
use ash::prelude::*;
use crate::assets::ImageAssetData;
use crate::assets::ShaderAssetData;
use crate::assets::{
    PipelineAssetData, MaterialAssetData, MaterialInstanceAssetData, RenderpassAssetData,
};
use crate::vk_description::{SwapchainSurfaceInfo, ImageAspectFlags};
use atelier_assets::loader::handle::Handle;
use std::mem::ManuallyDrop;
use crate::{
    vk_description as dsc, ResourceArc, DescriptorSetLayoutResource, PipelineLayoutResource,
    PipelineResource, ImageViewResource, DescriptorSetArc, DescriptorSetAllocatorMetrics,
    GenericLoader, BufferAssetData, AssetLookupSet, DynResourceAllocatorSet, LoadQueues,
    AssetLookup, MaterialPassSwapchainResources, SlotNameLookup, SlotLocation, PipelineCreateData,
    DynPassMaterialInstance, DynDescriptorSet, DescriptorSetAllocatorRef, DynMaterialInstance,
    DescriptorSetAllocatorProvider,
};
use crate::assets::{
    ShaderAsset, PipelineAsset, RenderpassAsset, MaterialAsset, MaterialInstanceAsset, ImageAsset,
    BufferAsset, MaterialPass,
};
use super::dyn_resource_allocator;
use super::resource_lookup;

use atelier_assets::loader::AssetLoadOp;
use atelier_assets::loader::handle::AssetHandle;
use std::sync::{Arc, Mutex};
use crate::resources::asset_lookup::LoadedAssetMetrics;
use crate::resources::dyn_resource_allocator::DynResourceAllocatorManagerSet;
use crate::resources::descriptor_sets;
use crate::resources::resource_lookup::ResourceLookupSet;
use crate::resources::load_queue::LoadQueueSet;
use crate::resources::swapchain_management::ActiveSwapchainSurfaceInfoSet;
use crate::resources::descriptor_sets::{DescriptorSetAllocator, DescriptorSetAllocatorManager};
use crate::resources::upload::{UploadManager, ImageUploadOpResult, BufferUploadOpResult};
use crossbeam_channel::Sender;
use crate::assets::MaterialPassDataRenderpassRef;

//TODO: Support descriptors that can be different per-view
//TODO: Support dynamic descriptors tied to command buffers?
//TODO: Support data inheritance for descriptors

// Information about a pipeline for a particular swapchain, resources may or may not be shared
// across swapchains depending on if they are the same size/format
pub struct PipelineSwapchainInfo {
    pub descriptor_set_layouts: Vec<ResourceArc<DescriptorSetLayoutResource>>,
    pub pipeline_layout: ResourceArc<PipelineLayoutResource>,
    pub pipeline: ResourceArc<PipelineResource>,
}

// Information about a single loaded image
pub struct ImageInfo {
    pub image: ResourceArc<VkImageRaw>,
    pub image_view: ResourceArc<ImageViewResource>,
}

// Information about a single descriptor set
pub struct DescriptorSetInfo {
    pub descriptor_set_layout_def: dsc::DescriptorSetLayout,
    pub descriptor_set_layout: ResourceArc<DescriptorSetLayoutResource>,
}

// Information about a descriptor set for a particular frame. Descriptor sets may be updated
// every frame and we rotate through them, so this information must not be persisted across frames
pub struct MaterialInstanceInfo {
    pub descriptor_sets: Arc<Vec<Vec<DescriptorSetArc>>>,
}

#[derive(Debug)]
pub struct ResourceManagerMetrics {
    pub dyn_resource_metrics: dyn_resource_allocator::ResourceMetrics,
    pub resource_metrics: resource_lookup::ResourceMetrics,
    pub loaded_asset_metrics: LoadedAssetMetrics,
    pub resource_descriptor_sets_metrics: DescriptorSetAllocatorMetrics,
}

pub struct ResourceManagerLoaders {
    pub shader_loader: GenericLoader<ShaderAssetData, ShaderAsset>,
    pub pipeline_loader: GenericLoader<PipelineAssetData, PipelineAsset>,
    pub renderpass_loader: GenericLoader<RenderpassAssetData, RenderpassAsset>,
    pub material_loader: GenericLoader<MaterialAssetData, MaterialAsset>,
    pub material_instance_loader: GenericLoader<MaterialInstanceAssetData, MaterialInstanceAsset>,
    pub image_loader: GenericLoader<ImageAssetData, ImageAsset>,
    pub buffer_loader: GenericLoader<BufferAssetData, BufferAsset>,
}

pub struct ResourceManager {
    dyn_resources: DynResourceAllocatorManagerSet,
    resources: ResourceLookupSet,
    loaded_assets: AssetLookupSet,
    load_queues: LoadQueueSet,
    swapchain_surfaces: ActiveSwapchainSurfaceInfoSet,
    resource_descriptor_sets: DescriptorSetAllocator,
    descriptor_set_allocator: DescriptorSetAllocatorManager,
    upload_manager: UploadManager,
}

impl ResourceManager {
    pub fn resources(&self) -> &ResourceLookupSet {
        &self.resources
    }

    pub fn resources_mut(&mut self) -> &mut ResourceLookupSet {
        &mut self.resources
    }

    #[allow(dead_code)]
    pub(super) fn assets(&self) -> &AssetLookupSet {
        &self.loaded_assets
    }

    #[allow(dead_code)]
    pub(super) fn assets_mut(&mut self) -> &mut AssetLookupSet {
        &mut self.loaded_assets
    }

    pub fn new(device_context: &VkDeviceContext) -> Self {
        ResourceManager {
            dyn_resources: DynResourceAllocatorManagerSet::new(
                device_context,
                renderer_shell_vulkan::MAX_FRAMES_IN_FLIGHT as u32,
            ),
            resources: ResourceLookupSet::new(
                device_context,
                renderer_shell_vulkan::MAX_FRAMES_IN_FLIGHT as u32,
            ),
            loaded_assets: Default::default(),
            load_queues: Default::default(),
            swapchain_surfaces: Default::default(),
            resource_descriptor_sets: DescriptorSetAllocator::new(device_context),
            descriptor_set_allocator: DescriptorSetAllocatorManager::new(device_context),
            upload_manager: UploadManager::new(device_context),
        }
    }

    pub fn loaded_assets(&self) -> &AssetLookupSet {
        &self.loaded_assets
    }

    pub fn create_shader_loader(&self) -> GenericLoader<ShaderAssetData, ShaderAsset> {
        self.load_queues.shader_modules.create_loader()
    }

    pub fn create_pipeline_loader(&self) -> GenericLoader<PipelineAssetData, PipelineAsset> {
        self.load_queues.graphics_pipelines.create_loader()
    }

    pub fn create_renderpass_loader(&self) -> GenericLoader<RenderpassAssetData, RenderpassAsset> {
        self.load_queues.renderpasses.create_loader()
    }

    pub fn create_material_loader(&self) -> GenericLoader<MaterialAssetData, MaterialAsset> {
        self.load_queues.materials.create_loader()
    }

    pub fn create_material_instance_loader(
        &self
    ) -> GenericLoader<MaterialInstanceAssetData, MaterialInstanceAsset> {
        self.load_queues.material_instances.create_loader()
    }

    pub fn create_image_loader(&self) -> GenericLoader<ImageAssetData, ImageAsset> {
        self.load_queues.images.create_loader()
    }

    pub fn create_buffer_loader(&self) -> GenericLoader<BufferAssetData, BufferAsset> {
        self.load_queues.buffers.create_loader()
    }

    pub fn create_loaders(&self) -> ResourceManagerLoaders {
        ResourceManagerLoaders {
            shader_loader: self.create_shader_loader(),
            pipeline_loader: self.create_pipeline_loader(),
            renderpass_loader: self.create_renderpass_loader(),
            material_loader: self.create_material_loader(),
            material_instance_loader: self.create_material_instance_loader(),
            image_loader: self.create_image_loader(),
            buffer_loader: self.create_buffer_loader(),
        }
    }

    pub fn create_dyn_resource_allocator_set(&self) -> DynResourceAllocatorSet {
        self.dyn_resources.create_allocator_set()
    }

    pub fn create_descriptor_set_allocator(&self) -> DescriptorSetAllocatorRef {
        self.descriptor_set_allocator.get_allocator()
    }

    pub fn create_descriptor_set_allocator_provider(&self) -> DescriptorSetAllocatorProvider {
        self.descriptor_set_allocator.create_allocator_provider()
    }

    pub fn get_image_info(
        &self,
        handle: &Handle<ImageAsset>,
    ) -> Option<ImageInfo> {
        self.loaded_assets
            .images
            .get_committed(handle.load_handle())
            .map(|loaded_image| ImageInfo {
                image: loaded_image.image.clone(),
                image_view: loaded_image.image_view.clone(),
            })
    }

    pub fn get_descriptor_set_info(
        &self,
        handle: &Handle<MaterialAsset>,
        pass_index: usize,
        layout_index: usize,
    ) -> DescriptorSetInfo {
        let resource = self
            .loaded_assets
            .materials
            .get_committed(handle.load_handle())
            .unwrap();

        let descriptor_set_layout_def = resource.passes[pass_index]
            .pipeline_create_data
            .pipeline_layout_def()
            .descriptor_set_layouts[layout_index]
            .clone();
        let descriptor_set_layout =
            resource.passes[pass_index].descriptor_set_layouts[layout_index].clone();

        DescriptorSetInfo {
            descriptor_set_layout_def,
            descriptor_set_layout,
        }
    }

    pub fn get_pipeline_info(
        &self,
        handle: &Handle<MaterialAsset>,
        swapchain: &SwapchainSurfaceInfo,
        pass_index: usize,
    ) -> PipelineSwapchainInfo {
        let resource = self
            .loaded_assets
            .materials
            .get_committed(handle.load_handle())
            .unwrap();

        let swapchain_index = self
            .swapchain_surfaces
            .ref_counts
            .get(swapchain)
            .unwrap()
            .index;

        let pipeline = {
            let per_swapchain_data = resource.passes[pass_index]
                .per_swapchain_data
                .lock()
                .unwrap();
            per_swapchain_data[swapchain_index].pipeline.clone()
        };

        PipelineSwapchainInfo {
            descriptor_set_layouts: resource.passes[pass_index].descriptor_set_layouts.clone(),
            pipeline_layout: resource.passes[pass_index].pipeline_layout.clone(),
            pipeline,
        }
    }

    pub fn get_material_instance_info(
        &self,
        handle: &Handle<MaterialInstanceAsset>,
    ) -> MaterialInstanceInfo {
        // Get the material instance
        let resource = self
            .loaded_assets
            .material_instances
            .get_committed(handle.load_handle())
            .unwrap();

        MaterialInstanceInfo {
            descriptor_sets: resource.inner.material_descriptor_sets.clone(),
        }
    }

    pub fn add_swapchain(
        &mut self,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    ) -> VkResult<()> {
        log::info!("add_swapchain {:?}", swapchain_surface_info);
        self.swapchain_surfaces.add(
            &swapchain_surface_info,
            &mut self.loaded_assets,
            &mut self.resources,
        )
    }

    pub fn remove_swapchain(
        &mut self,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    ) {
        log::info!("remove_swapchain {:?}", swapchain_surface_info);
        self.swapchain_surfaces
            .remove(swapchain_surface_info, &mut self.loaded_assets);
    }

    // Call whenever you want to handle assets loading/unloading
    pub fn update_resources(&mut self) -> VkResult<()> {
        self.process_shader_load_requests();
        self.process_pipeline_load_requests();
        self.process_renderpass_load_requests();
        self.process_material_load_requests();
        self.process_material_instance_load_requests();
        self.process_image_load_requests()?;
        self.process_buffer_load_requests()?;

        self.upload_manager.update()?;

        //self.dump_stats();

        Ok(())
    }

    // Call just before rendering
    pub fn on_begin_frame(&mut self) -> VkResult<()> {
        self.resource_descriptor_sets.flush_changes()
    }

    pub fn on_frame_complete(&mut self) -> VkResult<()> {
        self.resources.on_frame_complete()?;
        self.dyn_resources.on_frame_complete()?;
        self.resource_descriptor_sets.on_frame_complete();
        self.descriptor_set_allocator.on_frame_complete();
        Ok(())
    }

    pub fn metrics(&self) -> ResourceManagerMetrics {
        let dyn_resource_metrics = self.dyn_resources.metrics();
        let resource_metrics = self.resources.metrics();
        let loaded_asset_metrics = self.loaded_assets.metrics();
        let resource_descriptor_sets_metrics = self.resource_descriptor_sets.metrics();

        ResourceManagerMetrics {
            dyn_resource_metrics,
            resource_metrics,
            loaded_asset_metrics,
            resource_descriptor_sets_metrics,
        }
    }

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

    fn process_pipeline_load_requests(&mut self) {
        for request in self.load_queues.graphics_pipelines.take_load_requests() {
            log::trace!("Create pipeline {:?}", request.load_handle);
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
            log::info!(
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

    fn finish_load_image(
        &mut self,
        image: VkImage,
    ) -> VkResult<ImageAsset> {
        let format = image.format.into();
        let mip_level_count = image.mip_level_count;

        let (image_key, image_arc) = self.resources.insert_image(ManuallyDrop::new(image));

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
            .resources
            .get_or_create_image_view(image_key, &image_view_meta)?;

        Ok(ImageAsset {
            image_key,
            image: image_arc,
            image_view,
        })
    }

    fn finish_load_buffer(
        &mut self,
        buffer: VkBuffer,
    ) -> VkResult<BufferAsset> {
        let (buffer_key, buffer) = self.resources.insert_buffer(ManuallyDrop::new(buffer));

        Ok(BufferAsset { buffer_key, buffer })
    }

    fn load_shader_module(
        &mut self,
        shader_module: &ShaderAssetData,
    ) -> VkResult<ShaderAsset> {
        let shader_module = self
            .resources
            .get_or_create_shader_module(&shader_module.shader)?;
        Ok(ShaderAsset { shader_module })
    }

    fn load_graphics_pipeline(
        &mut self,
        pipeline_asset: PipelineAssetData,
    ) -> VkResult<PipelineAsset> {
        Ok(PipelineAsset {
            pipeline_asset: Arc::new(pipeline_asset),
        })
    }

    fn load_renderpass(
        &mut self,
        renderpass_asset: RenderpassAssetData,
    ) -> VkResult<RenderpassAsset> {
        Ok(RenderpassAsset {
            data: Arc::new(renderpass_asset),
        })
    }

    fn load_material(
        &mut self,
        material_asset: &MaterialAssetData,
    ) -> VkResult<MaterialAsset> {
        let mut passes = Vec::with_capacity(material_asset.passes.len());

        for pass in &material_asset.passes {
            let loaded_pipeline_asset = self
                .loaded_assets
                .graphics_pipelines
                .get_latest(pass.pipeline.load_handle())
                .unwrap();
            let pipeline_asset = loaded_pipeline_asset.pipeline_asset.clone();

            let renderpass_handle = match &pass.renderpass {
                MaterialPassDataRenderpassRef::Asset(asset) => asset,
                MaterialPassDataRenderpassRef::LookupByPhaseName => unimplemented!(),
            };

            let loaded_renderpass_asset = self
                .loaded_assets
                .renderpasses
                .get_latest(renderpass_handle.load_handle())
                .unwrap();
            let renderpass_asset = loaded_renderpass_asset.data.clone();

            let shader_hashes: Vec<_> = pass
                .shaders
                .iter()
                .map(|shader| {
                    let shader_module = self
                        .loaded_assets
                        .shader_modules
                        .get_latest(shader.shader_module.load_handle())
                        .unwrap();
                    shader_module.shader_module.get_hash().into()
                })
                .collect();

            let swapchain_surface_infos = self.swapchain_surfaces.unique_swapchain_infos().clone();
            let pipeline_create_data = PipelineCreateData::new(
                self,
                &*pipeline_asset,
                &*renderpass_asset,
                pass,
                shader_hashes,
            )?;

            // Will contain the vulkan resources being created per swapchain
            let mut per_swapchain_data = Vec::with_capacity(swapchain_surface_infos.len());

            // Create the pipeline objects
            for swapchain_surface_info in swapchain_surface_infos {
                let pipeline = self.resources.get_or_create_graphics_pipeline(
                    &pipeline_create_data,
                    &swapchain_surface_info,
                )?;

                per_swapchain_data.push(MaterialPassSwapchainResources { pipeline });
            }

            // Create a lookup of the slot names
            let mut pass_slot_name_lookup: SlotNameLookup = Default::default();
            for (layout_index, layout) in pass
                .shader_interface
                .descriptor_set_layouts
                .iter()
                .enumerate()
            {
                for (binding_index, binding) in
                    layout.descriptor_set_layout_bindings.iter().enumerate()
                {
                    pass_slot_name_lookup
                        .entry(binding.slot_name.clone())
                        .or_default()
                        .push(SlotLocation {
                            layout_index: layout_index as u32,
                            binding_index: binding_index as u32,
                            //array_index: 0
                        });
                }
            }

            passes.push(MaterialPass {
                descriptor_set_layouts: pipeline_create_data.descriptor_set_layout_arcs().clone(),
                pipeline_layout: pipeline_create_data.pipeline_layout().clone(),
                shader_modules: pipeline_create_data.shader_module_arcs().clone(),
                per_swapchain_data: Mutex::new(per_swapchain_data),
                pipeline_create_data,
                shader_interface: pass.shader_interface.clone(),
                pass_slot_name_lookup: Arc::new(pass_slot_name_lookup),
            })
        }

        Ok(MaterialAsset {
            passes: Arc::new(passes),
        })
    }

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
            let pass_descriptor_set_writes =
                descriptor_sets::create_write_sets_for_material_instance_pass(
                    pass,
                    &material_instance_asset.slot_assignments,
                    &self.loaded_assets,
                    &mut self.resources,
                )?;

            log::trace!(
                "load_material_instance descriptor set write\n{:#?}",
                pass_descriptor_set_writes
            );

            // Save the
            material_instance_descriptor_set_writes.push(pass_descriptor_set_writes.clone());

            // This will contain the descriptor sets created for this pass, one for each set within the pass
            let mut pass_descriptor_sets = Vec::with_capacity(pass_descriptor_set_writes.len());

            //
            // Register the writes into the correct descriptor set pools
            //
            //let layouts = pass.pipeline_create_data.pipeline_layout.iter().zip(&pass.pipeline_create_data.pipeline_layout_def);
            for (layout_index, layout_writes) in pass_descriptor_set_writes.into_iter().enumerate()
            {
                let descriptor_set = self.resource_descriptor_sets.create_descriptor_set(
                    &pass.pipeline_create_data.descriptor_set_layout_arcs()[layout_index],
                    layout_writes,
                )?;

                pass_descriptor_sets.push(descriptor_set);
            }

            material_descriptor_sets.push(pass_descriptor_sets);
        }

        log::trace!("Loaded material\n{:#?}", material_descriptor_sets);

        // Put these in an arc because
        let material_descriptor_sets = Arc::new(material_descriptor_sets);
        Ok(MaterialInstanceAsset::new(
            material_instance_asset.material.clone(),
            material_descriptor_sets,
            material_instance_asset.slot_assignments.clone(),
            material_instance_descriptor_set_writes,
        ))
    }

    pub fn create_dyn_descriptor_set_uninitialized(
        &self,
        descriptor_set_allocator: &mut DescriptorSetAllocator,
        descriptor_set_layout: &ResourceArc<DescriptorSetLayoutResource>,
    ) -> VkResult<DynDescriptorSet> {
        descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(descriptor_set_layout)
    }

    pub fn create_dyn_pass_material_instance_uninitialized(
        &self,
        descriptor_set_allocator: &mut DescriptorSetAllocator,
        material: Handle<MaterialAsset>,
        pass_index: u32,
    ) -> VkResult<DynPassMaterialInstance> {
        let material_asset = self
            .loaded_assets
            .materials
            .get_latest(material.load_handle())
            .unwrap();

        descriptor_set_allocator.create_dyn_pass_material_instance_uninitialized(
            &material_asset.passes[pass_index as usize],
        )
    }

    pub fn create_dyn_pass_material_instance_from_asset(
        &mut self,
        descriptor_set_allocator: &mut DescriptorSetAllocator,
        material_instance: Handle<MaterialInstanceAsset>,
        pass_index: u32,
    ) -> VkResult<DynPassMaterialInstance> {
        let material_instance_asset = self
            .loaded_assets
            .material_instances
            .get_committed(material_instance.load_handle())
            .unwrap();

        let material_asset = self
            .loaded_assets
            .materials
            .get_latest(material_instance_asset.inner.material.load_handle())
            .unwrap();

        descriptor_set_allocator.create_dyn_pass_material_instance_from_asset(
            &material_asset.passes[pass_index as usize],
            material_instance_asset.inner.descriptor_set_writes[pass_index as usize].clone(),
        )
    }

    pub fn create_dyn_material_instance_uninitialized(
        &self,
        descriptor_set_allocator: &mut DescriptorSetAllocator,
        material: Handle<MaterialAsset>,
    ) -> VkResult<DynMaterialInstance> {
        let material_asset = self
            .loaded_assets
            .materials
            .get_latest(material.load_handle())
            .unwrap();

        descriptor_set_allocator.create_dyn_material_instance_uninitialized(material_asset)
    }

    pub fn create_dyn_material_instance_from_asset(
        &self, // mut required because the asset may describe a sampler that needs to be created
        descriptor_set_allocator: &mut DescriptorSetAllocator,
        material_instance: Handle<MaterialInstanceAsset>,
    ) -> VkResult<DynMaterialInstance> {
        let material_instance_asset = self
            .loaded_assets
            .material_instances
            .get_committed(material_instance.load_handle())
            .unwrap();

        let material_asset = self
            .loaded_assets
            .materials
            .get_latest(material_instance_asset.inner.material.load_handle())
            .unwrap();

        descriptor_set_allocator
            .create_dyn_material_instance_from_asset(material_asset, material_instance_asset)
    }
}

impl Drop for ResourceManager {
    fn drop(&mut self) {
        log::info!("Cleaning up resource manager");
        log::trace!("Resource Manager Metrics:\n{:#?}", self.metrics());

        // Wipe out any loaded assets. This will potentially drop ref counts on resources
        self.loaded_assets.destroy();

        // Drop all descriptors. These bind to raw resources, so we need to drop them before
        // dropping resources
        self.resource_descriptor_sets.destroy().unwrap();
        self.descriptor_set_allocator.destroy().unwrap();

        // Now drop all resources with a zero ref count and warn for any resources that remain
        self.resources.destroy().unwrap();
        self.dyn_resources.destroy().unwrap();

        log::info!("Dropping resource manager");
        log::trace!("Resource Manager Metrics:\n{:#?}", self.metrics());
    }
}
