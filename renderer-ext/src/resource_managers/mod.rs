use crate::upload::{UploadQueue, ImageUploadOpResult, BufferUploadOpResult, PendingImageUpload, UploadOp};
use crossbeam_channel::{Sender, Receiver};
use renderer_shell_vulkan::{VkDeviceContext, VkImage};
use ash::prelude::*;
use ash::vk;
use crate::pipeline::image::ImageAsset;
use crate::image_utils::DecodedTexture;
use crate::pipeline::shader::ShaderAsset;
use crate::pipeline::pipeline::{PipelineAsset, MaterialAsset, MaterialInstanceAsset, MaterialPass};
use crate::pipeline_description::SwapchainSurfaceInfo;
use atelier_assets::loader::handle::Handle;
use std::mem::ManuallyDrop;
use fnv::FnvHashMap;
use crate::pipeline_description as dsc;
use atelier_assets::loader::AssetLoadOp;
use atelier_assets::loader::LoadHandle;
use atelier_assets::loader::handle::AssetHandle;


mod resource_lookup;
use resource_lookup::ResourceArc;
use resource_lookup::WeakResourceArc;
use resource_lookup::ResourceHash;
use resource_lookup::ResourceLookupSet;
use resource_lookup::ResourceMetrics;

mod load_queue;
use load_queue::LoadQueues;
use load_queue::GenericLoadHandler;
use load_queue::LoadRequest;
use load_queue::LoadQueueSet;

mod swapchain_management;
use swapchain_management::ActiveSwapchainSurfaceInfoSet;

mod asset_lookup;
use asset_lookup::LoadedImage;
use asset_lookup::LoadedShaderModule;
use asset_lookup::LoadedMaterialInstance;
use asset_lookup::LoadedMaterial;
use asset_lookup::LoadedMaterialPass;
use asset_lookup::LoadedGraphicsPipeline;
use asset_lookup::LoadedAssetLookupSet;
use asset_lookup::AssetLookup;
use asset_lookup::SlotLocation;

mod descriptor_sets;
use descriptor_sets::RegisteredDescriptorSetPoolManager;
use descriptor_sets::DescriptorSetArc;
use descriptor_sets::DescriptorSetWrite;
use descriptor_sets::DescriptorSetWriteImage;
use descriptor_sets::DescriptorSetWriteBuffer;
use descriptor_sets::RegisteredDescriptorSetPoolStats;
use descriptor_sets::RegisteredDescriptorSetPoolManagerStats;

//use descriptor_sets::
struct UploadManager {
    upload_queue: UploadQueue,

    image_upload_result_tx: Sender<ImageUploadOpResult>,
    image_upload_result_rx: Receiver<ImageUploadOpResult>,

    buffer_upload_result_tx: Sender<BufferUploadOpResult>,
    buffer_upload_result_rx: Receiver<BufferUploadOpResult>,
}

impl UploadManager {
    pub fn new(device_context: &VkDeviceContext) -> Self {
        let (image_upload_result_tx, image_upload_result_rx) = crossbeam_channel::unbounded();
        let (buffer_upload_result_tx, buffer_upload_result_rx) = crossbeam_channel::unbounded();

        UploadManager {
            upload_queue: UploadQueue::new(device_context),
            image_upload_result_rx,
            image_upload_result_tx,
            buffer_upload_result_rx,
            buffer_upload_result_tx
        }
    }

    pub fn update(&mut self) -> VkResult<()> {
        self.upload_queue.update()
    }

    pub fn upload_image(&self, request: LoadRequest<ImageAsset>) -> VkResult<()> {
        let decoded_texture = DecodedTexture {
            width: request.asset.width,
            height: request.asset.height,
            data: request.asset.data,
        };

        self.upload_queue.pending_image_tx().send(PendingImageUpload {
            load_op: request.load_op,
            upload_op: UploadOp::new(request.load_handle, self.image_upload_result_tx.clone()),
            texture: decoded_texture
        }).map_err(|err| {
            log::error!("Could not enqueue image upload");
            vk::Result::ERROR_UNKNOWN
        })
    }
}














pub struct PipelineInfo {
    pub descriptor_set_layouts: Vec<ResourceArc<vk::DescriptorSetLayout>>,
    pub pipeline_layout: ResourceArc<vk::PipelineLayout>,
    pub renderpass: ResourceArc<vk::RenderPass>,
    pub pipeline: ResourceArc<vk::Pipeline>,
}

pub struct CurrentFramePassInfo {
    pub descriptor_sets: Vec<vk::DescriptorSet>
}

pub struct ResourceManager {
    device_context: VkDeviceContext,

    resources: ResourceLookupSet,
    loaded_assets: LoadedAssetLookupSet,
    load_queues: LoadQueueSet,
    swapchain_surfaces: ActiveSwapchainSurfaceInfoSet,
    registered_descriptor_sets: RegisteredDescriptorSetPoolManager,
    upload_manager: UploadManager,
}

impl ResourceManager {
    pub fn new(device_context: &VkDeviceContext) -> Self {
        ResourceManager {
            device_context: device_context.clone(),
            resources: ResourceLookupSet::new(device_context,renderer_shell_vulkan::MAX_FRAMES_IN_FLIGHT as u32),
            loaded_assets: Default::default(),
            load_queues: Default::default(),
            swapchain_surfaces: Default::default(),
            registered_descriptor_sets: RegisteredDescriptorSetPoolManager::new(device_context),
            upload_manager: UploadManager::new(device_context),
        }
    }

    pub fn create_shader_load_handler(&self) -> GenericLoadHandler<ShaderAsset> {
        self.load_queues.shader_modules.create_load_handler()
    }

    pub fn create_pipeline_load_handler(&self) -> GenericLoadHandler<PipelineAsset> {
        self.load_queues.graphics_pipelines2.create_load_handler()
    }

    pub fn create_material_load_handler(&self) -> GenericLoadHandler<MaterialAsset> {
        self.load_queues.materials.create_load_handler()
    }

    pub fn create_material_instance_load_handler(&self) -> GenericLoadHandler<MaterialInstanceAsset> {
        self.load_queues.material_instances.create_load_handler()
    }

    pub fn create_image_load_handler(&self) -> GenericLoadHandler<ImageAsset> {
        self.load_queues.images.create_load_handler()
    }

    pub fn get_pipeline_info(
        &self,
        handle: &Handle<MaterialAsset>,
        swapchain: &SwapchainSurfaceInfo,
        pass_index: usize,
    ) -> PipelineInfo {
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

        PipelineInfo {
            descriptor_set_layouts: resource.passes[pass_index].descriptor_set_layouts.clone(),
            pipeline_layout: resource.passes[pass_index].pipeline_layout.clone(),
            renderpass: resource.passes[pass_index].render_passes[swapchain_index].clone(),
            pipeline: resource.passes[pass_index].pipelines[swapchain_index].clone(),
        }
    }

    pub fn get_current_frame_pass_info(
        &self,
        handle: &Handle<MaterialInstanceAsset>,
        pass_index: usize,
    ) -> CurrentFramePassInfo {
        let resource = self
            .loaded_assets
            .material_instances
            .get_committed(handle.load_handle())
            .unwrap();
        // let swapchain_index = self
        //     .swapchain_surfaces
        //     .ref_counts
        //     .get(swapchain)
        //     .unwrap()
        //     .index;

        // Get the current pass
        // Get the descriptor sets within the pass (one per layout)
        // Map the DescriptorSetArc to a vk::DescriptorSet
        let descriptor_sets : Vec<_> = resource.material_descriptor_sets[pass_index].iter()
            .map(|x| self.registered_descriptor_sets.descriptor_set(x))
            .collect();

        CurrentFramePassInfo {
            descriptor_sets
        }


        // PipelineInfo {
        //     descriptor_set_layouts: resource.passes[pass_index].descriptor_set_layouts.clone(),
        //     pipeline_layout: resource.passes[pass_index].pipeline_layout.clone(),
        //     renderpass: resource.passes[pass_index].render_passes[swapchain_index].clone(),
        //     pipeline: resource.passes[pass_index].pipelines[swapchain_index].clone(),
        // }
    }



    fn add_material_for_swapchain(
        //&mut self,
        resources: &mut ResourceLookupSet,
        swapchain_surface_info: &SwapchainSurfaceInfo,
        loaded_material: &mut LoadedMaterial
    ) -> VkResult<()> {
        for pass in &mut loaded_material.passes {
            let (renderpass, pipeline) = resources.get_or_create_graphics_pipeline(
                &pass.pipeline_create_data,
                swapchain_surface_info,
            )?;

            pass.render_passes.push(renderpass);
            pass.pipelines.push(pipeline);
        }

        Ok(())
    }

    pub fn add_swapchain(
        &mut self,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    ) -> VkResult<()> {
        log::info!("add_swapchain {:?}", swapchain_surface_info);
        // Add it
        if self.swapchain_surfaces.add(&swapchain_surface_info) {

            for (load_handle, loaded_asset) in
            &mut self.loaded_assets.materials.loaded_assets
            {
                if let Some(committed) = &mut loaded_asset.committed {
                    Self::add_material_for_swapchain(&mut self.resources, swapchain_surface_info, committed)?;
                }

                if let Some(uncommitted) = &mut loaded_asset.uncommitted {
                    Self::add_material_for_swapchain(&mut self.resources, swapchain_surface_info, uncommitted)?;
                }
            }
        }

        Ok(())
    }

    pub fn remove_swapchain(
        &mut self,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    ) {
        log::info!("remove_swapchain {:?}", swapchain_surface_info);
        //TODO: Common case is to destroy and re-create the same swapchain surface info, so we can
        // delay destroying until we also get an additional add/remove. If the next add call is
        // the same, we can avoid the remove entirely
        if let Some(remove_index) = self.swapchain_surfaces.remove(swapchain_surface_info) {
            for (_, loaded_asset) in &mut self.loaded_assets.materials.loaded_assets {
                if let Some(committed) = &mut loaded_asset.committed {
                    for pass in &mut committed.passes {
                        pass.render_passes.swap_remove(remove_index);
                        pass.pipelines.swap_remove(remove_index);
                    }
                }

                if let Some(uncommitted) = &mut loaded_asset.uncommitted {
                    for pass in &mut uncommitted.passes {
                        pass.render_passes.swap_remove(remove_index);
                        pass.pipelines.swap_remove(remove_index);
                    }
                }
            }
        } else {
            log::error!(
                "Received a remove swapchain without a matching add\n{:#?}",
                swapchain_surface_info
            );
        }
    }

    pub fn update(&mut self) -> VkResult<()> {
        self.process_shader_load_requests();
        self.process_pipeline_load_requests();
        self.process_material_load_requests();
        self.process_material_instance_load_requests();
        self.process_image_load_requests();

        self.registered_descriptor_sets.update();
        self.upload_manager.update()?;

        //self.dump_stats();

        Ok(())
    }

    fn dump_stats(&self) {
        let resource_metrics = self.resources.metrics();

        #[derive(Debug)]
        struct LoadedAssetCounts {
            shader_module_count: usize,
            pipeline_count: usize,
            material_count: usize,
            material_instance_count: usize,
            image_count: usize,
        }

        let loaded_asset_counts = LoadedAssetCounts {
            shader_module_count: self.loaded_assets.shader_modules.len(),
            pipeline_count: self.loaded_assets.graphics_pipelines2.len(),
            material_count: self.loaded_assets.materials.len(),
            material_instance_count: self.loaded_assets.material_instances.len(),
            image_count: self.loaded_assets.images.len(),
        };



        // #[derive(Debug)]
        // struct RegisteredDescriptorSetStats {
        //     pools: Vec<MaterialInstancePoolStats>
        // }
        //
        // let mut registered_descriptor_sets_stats = Vec::with_capacity(self.registered_descriptor_sets.pools.len());
        // for (hash, value) in &self.registered_descriptor_sets.pools {
        //     registered_descriptor_sets_stats.push(MaterialInstancePoolStats {
        //         hash: *hash,
        //         allocated_count: value.slab.allocated_count()
        //     });
        // }
        //
        // let registered_descriptor_sets_stats = RegisteredDescriptorSetStats {
        //     pools: registered_descriptor_sets_stats
        // };

        let registered_descriptor_sets_stats = self.registered_descriptor_sets.metrics();

        #[derive(Debug)]
        struct ResourceManagerMetrics {
            resource_metrics: ResourceMetrics,
            loaded_asset_counts: LoadedAssetCounts,
            registered_descriptor_sets_stats: RegisteredDescriptorSetPoolManagerStats
        }

        let metrics = ResourceManagerMetrics {
            resource_metrics,
            loaded_asset_counts,
            registered_descriptor_sets_stats
        };

        println!("Resource Manager Metrics:\n{:#?}", metrics);
    }

    fn process_shader_load_requests(&mut self) {
        for request in self.load_queues.shader_modules.take_load_requests() {
            println!("Create shader module {:?}", request.load_handle);
            let loaded_asset = self.load_shader_module(&request.asset);
            Self::handle_load_result(request.load_op, loaded_asset, &mut self.loaded_assets.shader_modules);
        }

        Self::handle_commit_requests(&mut self.load_queues.shader_modules, &mut self.loaded_assets.shader_modules);
        Self::handle_free_requests(&mut self.load_queues.shader_modules, &mut self.loaded_assets.shader_modules);
    }

    fn process_pipeline_load_requests(&mut self) {
        for request in self.load_queues.graphics_pipelines2.take_load_requests() {
            println!("Create pipeline {:?}", request.load_handle);
            let loaded_asset = self.load_graphics_pipeline(&request.asset);
            Self::handle_load_result(request.load_op, loaded_asset, &mut self.loaded_assets.graphics_pipelines2);
        }

        Self::handle_commit_requests(&mut self.load_queues.graphics_pipelines2, &mut self.loaded_assets.graphics_pipelines2);
        Self::handle_free_requests(&mut self.load_queues.graphics_pipelines2, &mut self.loaded_assets.graphics_pipelines2);
    }

    fn process_material_load_requests(&mut self) {
        for request in self.load_queues.materials.take_load_requests() {
            println!("Create material {:?}", request.load_handle);
            let loaded_asset = self.load_material(&request.asset);
            Self::handle_load_result(request.load_op, loaded_asset, &mut self.loaded_assets.materials);
        }

        Self::handle_commit_requests(&mut self.load_queues.materials, &mut self.loaded_assets.materials);
        Self::handle_free_requests(&mut self.load_queues.materials, &mut self.loaded_assets.materials);
    }

    fn process_material_instance_load_requests(&mut self) {
        for request in self.load_queues.material_instances.take_load_requests() {
            println!("Create material instance {:?}", request.load_handle);
            let loaded_asset = self.load_material_instance(&request.asset);
            Self::handle_load_result(request.load_op, loaded_asset, &mut self.loaded_assets.material_instances);
        }

        Self::handle_commit_requests(&mut self.load_queues.material_instances, &mut self.loaded_assets.material_instances);
        Self::handle_free_requests(&mut self.load_queues.material_instances, &mut self.loaded_assets.material_instances);
    }

    fn process_image_load_requests(&mut self) {
        for request in self.load_queues.images.take_load_requests() {
            //TODO: Route the request directly to the upload queue
            println!("Uploading image {:?}", request.load_handle);
            self.upload_manager.upload_image(request);
        }

        let results : Vec<_> = self.upload_manager.image_upload_result_rx.try_iter().collect();
        for result in results {
            match result {
                ImageUploadOpResult::UploadComplete(load_op, image) => {
                    let loaded_asset = self.finish_load_image(load_op.load_handle(), image);
                    Self::handle_load_result(load_op, loaded_asset, &mut self.loaded_assets.images);
                },
                ImageUploadOpResult::UploadError(load_handle) => {
                    // Don't need to do anything - the uploaded should have triggered an error on the load_op
                },
                ImageUploadOpResult::UploadDrop(load_handle) => {
                    // Don't need to do anything - the uploaded should have triggered an error on the load_op
                }
            }
        }

        Self::handle_commit_requests(&mut self.load_queues.images, &mut self.loaded_assets.images);
        Self::handle_free_requests(&mut self.load_queues.images, &mut self.loaded_assets.images);
    }

    fn handle_load_result<LoadedAssetT>(
        load_op: AssetLoadOp,
        loaded_asset: VkResult<LoadedAssetT>,
        asset_lookup: &mut AssetLookup<LoadedAssetT>
    ) {
        match loaded_asset {
            Ok(loaded_asset) => {
                asset_lookup.set_uncommitted(load_op.load_handle(), loaded_asset);
                load_op.complete()
            }
            Err(err) => {
                load_op.error(err);
            }
        }
    }

    fn handle_commit_requests<AssetT, LoadedAssetT>(
        load_queues: &mut LoadQueues<AssetT>,
        asset_lookup: &mut AssetLookup<LoadedAssetT>
    ) {
        for request in load_queues.take_commit_requests() {
            asset_lookup.commit(request.load_handle);
        }
    }

    fn handle_free_requests<AssetT, LoadedAssetT>(
        load_queues: &mut LoadQueues<AssetT>,
        asset_lookup: &mut AssetLookup<LoadedAssetT>
    ) {
        for request in load_queues.take_commit_requests() {
            asset_lookup.commit(request.load_handle);
        }
    }

    fn finish_load_image(
        &mut self,
        image_load_handle: LoadHandle,
        image: ManuallyDrop<VkImage>,
    ) -> VkResult<LoadedImage> {
        let image = self.resources.insert_image(image_load_handle, image);

        let image_view_meta = dsc::ImageViewMeta {
            view_type: dsc::ImageViewType::Type2D,
            format: dsc::Format::R8G8B8A8_UNORM,
            subresource_range: dsc::ImageSubresourceRange {
                aspect_mask: dsc::ImageAspectFlags::Color,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1
            },
            components: dsc::ComponentMapping {
                r: dsc::ComponentSwizzle::Identity,
                g: dsc::ComponentSwizzle::Identity,
                b: dsc::ComponentSwizzle::Identity,
                a: dsc::ComponentSwizzle::Identity,
            }
        };

        let image_view = self.resources.get_or_create_image_view(image_load_handle, &image_view_meta)?;

        Ok(LoadedImage {
            image,
            image_view
        })
    }

    fn load_shader_module(
        &mut self,
        shader_module: &ShaderAsset,
    ) -> VkResult<LoadedShaderModule> {
        let shader_module = self.resources.get_or_create_shader_module(&shader_module.shader)?;
        Ok(LoadedShaderModule {
            shader_module
        })
    }

    fn load_graphics_pipeline(
        &mut self,
        pipeline_asset: &PipelineAsset,
    ) -> VkResult<LoadedGraphicsPipeline> {
        Ok(LoadedGraphicsPipeline {
            pipeline_asset: pipeline_asset.clone()
        })
    }

    fn load_material(
        &mut self,
        material_asset: &MaterialAsset,
    ) -> VkResult<LoadedMaterial> {
        let mut passes = Vec::with_capacity(material_asset.passes.len());
        for pass in &material_asset.passes {
            let loaded_pipeline_asset = self.loaded_assets.graphics_pipelines2.get_latest(pass.pipeline.load_handle()).unwrap();
            let pipeline_asset = loaded_pipeline_asset.pipeline_asset.clone();

            let swapchain_surface_infos = self.swapchain_surfaces.unique_swapchain_infos().clone();
            let pipeline_create_data = PipelineCreateData::new(self, pipeline_asset, pass)?;

            // Will contain the vulkan resources being created per swapchain
            let mut render_passes =
                Vec::with_capacity(swapchain_surface_infos.len());
            let mut pipelines =
                Vec::with_capacity(swapchain_surface_infos.len());

            // Create the pipeline objects
            for swapchain_surface_info in swapchain_surface_infos
            {
                let (renderpass, pipeline) = self.resources.get_or_create_graphics_pipeline(
                    &pipeline_create_data,
                    &swapchain_surface_info,
                )?;
                render_passes.push(renderpass);
                pipelines.push(pipeline);
            }

            // Create a lookup of the slot names
            let mut pass_slot_name_lookup : FnvHashMap<String, Vec<SlotLocation>> = Default::default();
            for (layout_index, layout) in pass.shader_interface.descriptor_set_layouts.iter().enumerate() {
                for (binding_index, binding) in layout.descriptor_set_layout_bindings.iter().enumerate() {
                    pass_slot_name_lookup.entry(binding.slot_name.clone())
                        .or_default()
                        .push(SlotLocation {
                            layout_index: layout_index as u32,
                            binding_index: binding_index as u32,
                        });
                }
            }

            passes.push(LoadedMaterialPass {
                descriptor_set_layouts: pipeline_create_data.descriptor_set_layout_arcs.clone(),
                pipeline_layout: pipeline_create_data.pipeline_layout.clone(),
                shader_modules: pipeline_create_data.shader_module_arcs.clone(),
                render_passes,
                pipelines,
                pipeline_create_data,
                shader_interface: pass.shader_interface.clone(),
                pass_slot_name_lookup
            })
        }

        Ok(LoadedMaterial {
            passes,
        })
    }

    // fn begin_load_image(
    //     &mut self,
    //     request: LoadRequest<ImageAsset>
    // ) -> VkResult<LoadedMaterialInstance> {
    //     let decoded_image = DecodedTexture {
    //         width: request.asset.width,
    //         height: request.asset.height,
    //         data: request.asset.data,
    //     };
    //
    //     self.upload_manager.upload_queue.pending_image_tx().send(PendingImageUpload {
    //         load_op: request.load_op,
    //         upload_op: UploadOp::new(request.load_handle, self.upload_manager.image_upload_result_tx.clone()),
    //         texture: decoded_image
    //     }).map_err(|err| {
    //         log::error!("Could not enqueue image upload");
    //         vk::Result::ERROR_UNKNOWN
    //     })
    // }

    fn load_material_instance(
        &mut self,
        material_instance_asset: &MaterialInstanceAsset,
    ) -> VkResult<LoadedMaterialInstance> {
        // Find the material we will bind over, we need the metadata from it
        let material_asset = self.loaded_assets.materials.get_latest(material_instance_asset.material.load_handle()).unwrap();

        //TODO: Validate the material instance's slot names exist somewhere in the material

        // This will be references to descriptor sets. Indexed by pass, and then by set within the pass.
        let mut material_descriptor_sets = Vec::with_capacity(material_asset.passes.len());
        for pass in &material_asset.passes {
            // The metadata for the descriptor sets within this pass, one for each set within the pass
            let descriptor_set_layouts = &pass.shader_interface.descriptor_set_layouts;// &pass.pipeline_create_data.pipeline_layout_def.descriptor_set_layouts;

            // This will contain the descriptor sets created for this pass, one for each set within the pass
            let mut pass_descriptor_sets = Vec::with_capacity(descriptor_set_layouts.len());

            // This will contain the writes for the descriptor set. Their purpose is to store everything needed to create a vk::WriteDescriptorSet
            // struct. We will need to keep these around for a few frames.
            let mut pass_descriptor_set_writes = Vec::with_capacity(pass.shader_interface.descriptor_set_layouts.len());

            //
            // Build a "default" descriptor writer for every binding
            //
            for layout in &pass.shader_interface.descriptor_set_layouts {
                // This will contain the writes for this set
                let mut layout_descriptor_set_writes = Vec::with_capacity(layout.descriptor_set_layout_bindings.len());

                for (binding_index, binding) in layout.descriptor_set_layout_bindings.iter().enumerate() {
                    //TODO: Populate the writer for this binding
                    //TODO: Allocate a set from the pool
                    // //pub dst_pool_index: u32, // a slab key?
                    // //pub dst_set_index: u32,
                    // pub dst_binding: u32,
                    // pub dst_array_element: u32,
                    // pub descriptor_type: vk::DescriptorType,
                    // pub image_info: Option<Vec<DescriptorSetWriteImage>>,
                    // pub buffer_info: Option<Vec<DescriptorSetWriteBuffer>>,
                    layout_descriptor_set_writes.push(DescriptorSetWrite {
                        // pool index
                        // set index
                        dst_binding: binding_index as u32,
                        dst_array_element: 0,
                        descriptor_type: binding.descriptor_type.into(),
                        image_info: Default::default(),
                        buffer_info: Default::default(),
                    })
                }

                pass_descriptor_set_writes.push(layout_descriptor_set_writes);
            }

            //
            // Now modify the descriptor set writes to actually point at the things specified by the material
            //
            for slot in &material_instance_asset.slots {
                if let Some(slot_locations) = pass.pass_slot_name_lookup.get(&slot.slot_name) {
                    for location in slot_locations {
                        let mut writer = &mut pass_descriptor_set_writes[location.layout_index as usize][location.binding_index as usize];

                        let mut bind_samplers = false;
                        let mut bind_images = false;
                        match writer.descriptor_type {
                            dsc::DescriptorType::Sampler => {
                                bind_samplers = true;
                            },
                            dsc::DescriptorType::CombinedImageSampler => {
                                bind_samplers = true;
                                bind_images = true;
                            },
                            dsc::DescriptorType::SampledImage => {
                                bind_images = true;
                            },
                            _ => { unimplemented!() }
                        }

                        let mut write_image = DescriptorSetWriteImage {
                            image_view: None,
                            sampler: None
                        };

                        if bind_images {
                            if let Some(image) = &slot.image {
                                let loaded_image = self.loaded_assets.images.get_latest(image.load_handle()).unwrap();
                                write_image.image_view = Some(loaded_image.image_view.clone());
                            }
                        }

                        writer.image_info = vec![write_image];
                    }
                }
            }

            //
            // Register the writes into the correct descriptor set pools
            //
            //let layouts = pass.pipeline_create_data.pipeline_layout.iter().zip(&pass.pipeline_create_data.pipeline_layout_def);
            for (layout_index, layout_writes) in pass_descriptor_set_writes.into_iter().enumerate() {
                let descriptor_set = self.registered_descriptor_sets.insert(
                    &pass.pipeline_create_data.pipeline_layout_def.descriptor_set_layouts[layout_index],
                    pass.pipeline_create_data.descriptor_set_layout_arcs[layout_index].clone(),
                    layout_writes,
                )?;

                pass_descriptor_sets.push(descriptor_set);
            }

            material_descriptor_sets.push(pass_descriptor_sets);
        }

        println!("MATERIAL SET\n{:#?}", material_descriptor_sets);

        Ok(LoadedMaterialInstance {
            material_descriptor_sets
        })
    }
}









//
//
//

// struct PendingDescriptorSetWriteImage {
//
//     image: ResourceArc<vk::Image>,
// }
//
// struct PendingDescriptorSetWrite {
//
// }







impl Drop for ResourceManager {
    fn drop(&mut self) {
        unsafe {
            println!("Cleaning up resource manager");
            self.dump_stats();

            // Wipe out any loaded assets. This will potentially drop ref counts on resources
            self.loaded_assets.destroy();

            // Drop all descriptors
            self.registered_descriptor_sets.destroy();

            // Now drop all resources with a zero ref count and warn for any resources that remain
            self.resources.destroy();

            println!("Dropping resource manager");
            self.dump_stats();
        }
    }
}

// We have to create pipelines when pipeline assets load and when swapchains are added/removed.
// Gathering all the info to hash and create a pipeline is a bit involved so we share the code
// here
#[derive(Clone)]
pub struct PipelineCreateData {
    shader_module_metas: Vec<dsc::ShaderModuleMeta>,
    shader_module_load_handles: Vec<LoadHandle>,
    shader_module_arcs: Vec<ResourceArc<vk::ShaderModule>>,
    shader_module_vk_objs: Vec<vk::ShaderModule>,

    descriptor_set_layout_arcs: Vec<ResourceArc<vk::DescriptorSetLayout>>,

    fixed_function_state: dsc::FixedFunctionState,

    pipeline_layout_def: dsc::PipelineLayout,
    pipeline_layout: ResourceArc<vk::PipelineLayout>,

    renderpass: dsc::RenderPass,
}

impl PipelineCreateData {
    pub fn new(
        resource_manager: &mut ResourceManager,
        pipeline_asset: PipelineAsset,
        material_pass: &MaterialPass
    ) -> VkResult<Self> {
        //
        // Shader module metadata (required to create the pipeline key)
        //
        let mut shader_module_metas =
            Vec::with_capacity(material_pass.shaders.len());
        let mut shader_module_load_handles =
            Vec::with_capacity(material_pass.shaders.len());
        for stage in &material_pass.shaders {
            let shader_module_meta = dsc::ShaderModuleMeta {
                stage: stage.stage,
                entry_name: stage.entry_name.clone(),
            };
            shader_module_metas.push(shader_module_meta);
            shader_module_load_handles.push(stage.shader_module.load_handle());
        }

        //
        // Actual shader module resources (to create the pipeline)
        //
        let mut shader_module_arcs =
            Vec::with_capacity(material_pass.shaders.len());
        let mut shader_module_vk_objs =
            Vec::with_capacity(material_pass.shaders.len());
        for stage in &material_pass.shaders {
            let shader_module = resource_manager
                .loaded_assets
                .shader_modules
                .get_latest(stage.shader_module.load_handle())
                .unwrap();
            shader_module_arcs.push(shader_module.shader_module.clone());
            shader_module_vk_objs.push(shader_module.shader_module.get_raw());
        }

        //
        // Descriptor set layout
        //
        let mut descriptor_set_layout_arcs =
            Vec::with_capacity(material_pass.shader_interface.descriptor_set_layouts.len());
        let mut descriptor_set_layout_defs = Vec::with_capacity(material_pass.shader_interface.descriptor_set_layouts.len());
        for descriptor_set_layout_def in &material_pass.shader_interface.descriptor_set_layouts {
            let descriptor_set_layout_def = descriptor_set_layout_def.into();
            let descriptor_set_layout =
                resource_manager.resources.get_or_create_descriptor_set_layout(&descriptor_set_layout_def)?;
            descriptor_set_layout_arcs.push(descriptor_set_layout);
            descriptor_set_layout_defs.push(descriptor_set_layout_def);
        }

        //
        // Pipeline layout
        //
        let pipeline_layout_def = dsc::PipelineLayout {
            descriptor_set_layouts: descriptor_set_layout_defs,
            push_constant_ranges: material_pass.shader_interface.push_constant_ranges.clone(),
        };

        let pipeline_layout =
            resource_manager.resources.get_or_create_pipeline_layout(&pipeline_layout_def)?;

        let fixed_function_state = dsc::FixedFunctionState {
            vertex_input_state: material_pass.shader_interface.vertex_input_state.clone(),
            input_assembly_state: pipeline_asset.input_assembly_state,
            viewport_state: pipeline_asset.viewport_state,
            rasterization_state: pipeline_asset.rasterization_state,
            multisample_state: pipeline_asset.multisample_state,
            color_blend_state: pipeline_asset.color_blend_state,
            dynamic_state: pipeline_asset.dynamic_state,
        };

        Ok(PipelineCreateData {
            shader_module_metas,
            shader_module_load_handles,
            shader_module_arcs,
            shader_module_vk_objs,
            descriptor_set_layout_arcs,
            fixed_function_state,
            pipeline_layout_def,
            pipeline_layout,
            renderpass: pipeline_asset.renderpass,
        })
    }
}
