
use crate::pipeline_description as dsc;
use fnv::FnvHashMap;
use ash::vk;
use ash::version::DeviceV1_0;
use renderer_shell_vulkan::VkDeviceContext;
use ash::prelude::VkResult;
use std::collections::hash_map::Entry::Occupied;
use ash::vk::{PipelineDynamicStateCreateInfo};
use std::marker::PhantomData;
use atelier_assets::loader::LoadHandle;
use mopa::Any;
use std::ops::Deref;
use image::load;
use crossbeam_channel::Sender;
use std::hash::Hash;

/*
// A borrowed reference to a GPU resource
struct PipelineResourceRef<'a, DscT, GpuResourceT : Copy> {
    resource_manager: &'a PipelineResourceManager<DscT, GpuResourceT>,
    resource_hash: PipelineResourceHash,
    resource: GpuResourceT,
    phantom_data: PhantomData<DscT>
}

impl<'a, DscT, GpuResourceT : Copy> PipelineResourceRef<'a, DscT, GpuResourceT> {

}

impl<'a, DscT, GpuResourceT : Copy> Drop for PipelineResourceRef<'a, DscT, GpuResourceT> {
    fn drop(&mut self) {
        self.resource_manager.remove_hash_ref(self.resource_hash)
    }
}

impl<'a, DscT, GpuResourceT : Copy> Deref for PipelineResourceRef<'a, DscT, GpuResourceT> {
    type Target = GpuResourceT;

    fn deref(&self) -> &Self::Target {
        &self.resource
    }
}
*/
// Hash of a GPU resource
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct PipelineResourceHash(u64);

// Contains a set of GPU resources, deduplicated by hashing.
// - Every insert has to be associated with an asset LoadHandle
// - If two load handles try to insert the same value, only one GPU resource is created/stored
// - Any insertion must be matched by a remove using the same LoadHandle
// - The final LoadHandle remove will send the resource to something that drops it via drop_sender
struct PipelineResourceManagerHashState<GpuResourceT> {
    ref_count: std::sync::atomic::AtomicU32,
    vk_obj: GpuResourceT
}

#[derive(Default)]
pub struct PipelineResourceManager<DscT, GpuResourceT : Copy> {
    // Look up the hash of a resource by load handle. The resource could be cached here but since
    // they are reference counted, we would need to lookup by the hash anyways (or wrap the ref
    // count in an arc)
    by_load_handle: FnvHashMap<LoadHandle, PipelineResourceHash>,

    // Look up
    by_hash: FnvHashMap<PipelineResourceHash, PipelineResourceManagerHashState<GpuResourceT>>,

    // For debug purposes only to detect collisions
    values: FnvHashMap<PipelineResourceHash, DscT>,

    // // When we want to drop the resource, we send it via this channel. These will get flushed out
    // // and destroyed (potentially after a few frames have passed and it is no longer possible that
    // // it's being used from an in-flight submit
    // drop_sender: Sender<GpuResourceT>,

    phantom_data: PhantomData<DscT>
}

impl<DscT : PartialEq + Clone, GpuResourceT : Copy> PipelineResourceManager<DscT, GpuResourceT> {
    // Returns the resource
    fn get(
        &self,
        load_handle: LoadHandle
    ) -> Option<GpuResourceT> {
        let resource_hash = self.by_load_handle.get(&load_handle);
        if let Some(resource_hash) = resource_hash {
            let resource_hash = *resource_hash;
            let state = self.by_hash.get(&resource_hash).unwrap();
            Some(state.vk_obj)
        } else {
            None
        }
    }

    fn contains_resource(
        &self,
        hash: PipelineResourceHash,
    ) -> bool {
        self.by_hash.contains_key(&hash)
    }

    fn insert(
        &mut self,
        hash: PipelineResourceHash,
        load_handle: LoadHandle,
        dsc: &DscT,
        resource: GpuResourceT
    ) -> VkResult<GpuResourceT> {
        debug_assert!(!self.contains_resource(hash));

        // Insert the resource
        self.by_hash.insert(hash, PipelineResourceManagerHashState {
            ref_count: std::sync::atomic::AtomicU32::new(1),
            vk_obj: resource
        });

        self.by_load_handle.insert(load_handle, hash);
        self.values.insert(hash, dsc.clone());
        Ok(resource)
    }

    fn add_ref(
        &mut self,
        hash: PipelineResourceHash,
        load_handle: LoadHandle,
        dsc: &DscT,
    ) -> GpuResourceT {
        // Add ref count
        let state = self.by_hash.get(&hash).unwrap();
        state.ref_count.fetch_add(1, std::sync::atomic::Ordering::Acquire);

        // Store the load handle
        self.by_load_handle.insert(load_handle, hash);

        if let Some(value) = self.values.get(&hash) {
            assert!(*dsc == *value);
        } else {
            self.values.insert(hash, dsc.clone());
        }

        state.vk_obj
    }

    fn remove_ref(
        &mut self,
        load_handle: LoadHandle,
    ) -> Option<GpuResourceT> {
        match self.by_load_handle.get(&load_handle) {
            Some(hash) => {
                let hash = *hash;
                let (ref_count, vk_obj) = {
                    let state = self.by_hash.get(&hash).unwrap();

                    // Subtract one because fetch_sub returns the value in the state before it was subtracted
                    let ref_count = state.ref_count.fetch_sub(1, std::sync::atomic::Ordering::Release) - 1;
                    (ref_count, state.vk_obj)
                };

                self.by_load_handle.remove(&load_handle);

                if ref_count == 0 {
                    self.values.remove(&hash);
                    self.by_hash.remove(&hash);

                    // Return the underlying object if it is ready to be destroyed
                    Some(vk_obj)
                } else {
                    None
                }
            },
            None => {
                log::error!("A load handle was removed from a PipelineResourceManager but the passed in load handle was not found.");
                None
            }
        }
    }

    // Intended for use when destroying so that resources can be cleaned up
    fn take_all_resources(&mut self) -> Vec<GpuResourceT> {
        let resources = self.by_hash.iter().map(|(_, v)| v.vk_obj).collect();
        self.by_hash.clear();
        self.values.clear();
        self.by_load_handle.clear();
        resources
    }
}















// struct DescriptorSetLayoutState {
//     ref_count: u32,
//     vk_obj: vk::DescriptorSetLayout
// }
//
// struct PipelineLayoutState {
//     ref_count: u32,
//     vk_obj: vk::PipelineLayout
// }
//
// struct RenderPassState {
//     ref_count: u32,
//     vk_obj: vk::RenderPass
// }
//
// struct ShaderModuleState {
//     ref_count: u32,
//     vk_obj: vk::ShaderModule
// }
//
// struct GraphicsPipelineState {
//     ref_count: u32,
//     vk_obj: vk::Pipeline
// }




fn hash_resource_description<T : Hash>(t: &T) -> PipelineResourceHash {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    t.hash(&mut hasher);
    PipelineResourceHash(hasher.finish())
}



//TODO: Use hashes as keys instead of values (but maybe leave debug-only verification to make sure
// there aren't any hash collisions
pub struct PipelineManager {
    device_context: VkDeviceContext,
    //shader_modules: PipelineResourceManager<dsc::RenderPass, vk::ShaderModule>,
    descriptor_set_layouts: PipelineResourceManager<dsc::DescriptorSetLayout, vk::DescriptorSetLayout>,
    pipeline_layouts: PipelineResourceManager<dsc::PipelineLayout, vk::PipelineLayout>,
    renderpasses: PipelineResourceManager<dsc::RenderPass, vk::RenderPass>,
    graphics_pipelines: PipelineResourceManager<dsc::GraphicsPipeline, vk::Pipeline>,
    swapchain_surface_info: dsc::SwapchainSurfaceInfo,
}


impl PipelineManager {
    pub fn new(device_context: &VkDeviceContext, swapchain_surface_info: dsc::SwapchainSurfaceInfo) -> Self {
        PipelineManager {
            device_context: device_context.clone(),
            descriptor_set_layouts: Default::default(),
            pipeline_layouts: Default::default(),
            renderpasses: Default::default(),
            graphics_pipelines: Default::default(),
            swapchain_surface_info
        }
    }

    pub fn swapchain_surface_info(&self) -> &dsc::SwapchainSurfaceInfo {
        &self.swapchain_surface_info
    }

    pub fn load_descriptor_set_layout(
        &mut self,
        load_handle: LoadHandle,
        descriptor_set_layout: &dsc::DescriptorSetLayout,
    ) -> VkResult<vk::DescriptorSetLayout> {
        let hash = hash_resource_description(&descriptor_set_layout);
        if self.descriptor_set_layouts.contains_resource(hash) {
            Ok(self.descriptor_set_layouts.add_ref(hash, load_handle, descriptor_set_layout))
        } else {
            let resource =
                crate::pipeline_description::create_descriptor_set_layout(self.device_context.device(), descriptor_set_layout)?;
            self.descriptor_set_layouts.insert(hash, load_handle, descriptor_set_layout, resource);
            Ok(resource)
        }
    }

    pub fn load_pipeline_layout(
        &mut self,
        load_handle: LoadHandle,
        pipeline_layout: &dsc::PipelineLayout
    ) -> VkResult<vk::PipelineLayout> {

        let hash = hash_resource_description(&pipeline_layout);
        if self.pipeline_layouts.contains_resource(hash) {
            Ok(self.pipeline_layouts.add_ref(hash, load_handle, pipeline_layout))
        } else {
            let mut descriptor_set_layouts = Vec::with_capacity(pipeline_layout.descriptor_set_layouts.len());
            for descriptor_set_layout in &pipeline_layout.descriptor_set_layouts {
                descriptor_set_layouts.push(self.load_descriptor_set_layout(load_handle, descriptor_set_layout)?);
            }

            let resource =
                crate::pipeline_description::create_pipeline_layout(self.device_context.device(), pipeline_layout, &descriptor_set_layouts)?;
            self.pipeline_layouts.insert(hash, load_handle, pipeline_layout, resource);
            Ok(resource)
        }

        /*
        self.pipeline_layouts.insert_with(
            hash_resource_description(pipeline_layout),
            load_handle,
            pipeline_layout,
            || {
                let mut descriptor_set_layouts = Vec::with_capacity(pipeline_layout.descriptor_set_layouts.len());
                for descriptor_set_layout in &pipeline_layout.descriptor_set_layouts {
                    descriptor_set_layouts.push(self.load_descriptor_set_layout(load_handle, descriptor_set_layout)?);
                }

                let push_constant_ranges: Vec<_> = pipeline_layout.push_constant_ranges.iter()
                    .map(|push_constant_range| push_constant_range.as_builder().build())
                    .collect();

                let create_info = vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(descriptor_set_layouts.as_slice())
                    .push_constant_ranges(push_constant_ranges.as_slice());

                unsafe {
                    self.device_context.device().create_pipeline_layout(&*create_info, None)
                }
            }
        )
        */
    }

    pub fn load_renderpass(
        &mut self,
        load_handle: LoadHandle,
        renderpass: &dsc::RenderPass,
    ) -> VkResult<vk::RenderPass> {
        let hash = hash_resource_description(&renderpass);
        if self.renderpasses.contains_resource(hash) {
            Ok(self.renderpasses.add_ref(hash, load_handle, renderpass))
        } else {
            let resource =
                crate::pipeline_description::create_renderpass(self.device_context.device(), renderpass, &self.swapchain_surface_info)?;
            self.renderpasses.insert(hash, load_handle, renderpass, resource);
            Ok(resource)
        }

        /*
        self.renderpasses.insert_with(
            hash_resource_description(renderpass),
            load_handle,
            renderpass,
            || {
                let attachments : Vec<_> = renderpass.attachments.iter()
                    .map(|attachment| attachment.as_builder(&self.swapchain_surface_info).build())
                    .collect();

                // One vec per subpass
                let mut color_attachments : Vec<Vec<vk::AttachmentReference>> = Vec::with_capacity(renderpass.subpasses.len());
                let mut input_attachments : Vec<Vec<vk::AttachmentReference>> = Vec::with_capacity(renderpass.subpasses.len());
                let mut resolve_attachments : Vec<Vec<vk::AttachmentReference>> = Vec::with_capacity(renderpass.subpasses.len());

                // One element per subpass that has a depth stencil attachment specified
                let mut depth_stencil_attachments : Vec<vk::AttachmentReference> = Vec::with_capacity(renderpass.subpasses.len());

                let mut subpasses : Vec<_> = Vec::with_capacity(renderpass.subpasses.len());

                for subpass in &renderpass.subpasses {
                    color_attachments.push(subpass.color_attachments.iter().map(|attachment| attachment.as_builder().build()).collect());
                    input_attachments.push(subpass.input_attachments.iter().map(|attachment| attachment.as_builder().build()).collect());

                    // The resolve attachment array must be unused or of length == color attachments. If
                    // the number of subpass resolves doesn't match the color attachments, truncate or
                    // insert attachment references with AttachmentIndex::Unused
                    if subpass.resolve_attachments.len() > subpass.color_attachments.len() {
                        log::warn!("A renderpass definition has more resolve attachments than color attachments. The additional resolve attachments will be discarded");
                    }

                    let mut subpass_resolve_attachments : Vec<_> = subpass.resolve_attachments.iter().map(|attachment| attachment.as_builder().build()).collect();
                    if !subpass_resolve_attachments.is_empty() {
                        let unused_attachment = dsc::AttachmentReference {
                            attachment: dsc::AttachmentIndex::Unused,
                            layout: Default::default()
                        }.as_builder().build();
                        subpass_resolve_attachments.resize(color_attachments.len(), unused_attachment);
                    }
                    resolve_attachments.push(subpass_resolve_attachments);

                    let mut subpass_description_builder = vk::SubpassDescription::builder()
                        .pipeline_bind_point(subpass.pipeline_bind_point.into())
                        .color_attachments(color_attachments.last().unwrap())
                        .input_attachments(input_attachments.last().unwrap());

                    // Only specify resolve attachments if we have more than zero of them
                    {
                        let subpass_resolve_attachments = resolve_attachments.last().unwrap();
                        if subpass_resolve_attachments.len() > 0 {
                            subpass_description_builder = subpass_description_builder.resolve_attachments(subpass_resolve_attachments);
                        }
                    }

                    // Only specify a depth stencil attachment if we have one
                    if let Some(depth_stencil_attachment) = &subpass.depth_stencil_attachment {
                        depth_stencil_attachments.push(depth_stencil_attachment.as_builder().build());
                        subpass_description_builder = subpass_description_builder.depth_stencil_attachment(depth_stencil_attachments.last().unwrap());
                    }

                    let subpass_description = subpass_description_builder.build();

                    subpasses.push(subpass_description);
                }

                let dependencies : Vec<_> = renderpass.dependencies.iter()
                    .map(|dependency| dependency.as_builder().build())
                    .collect();

                let create_info = vk::RenderPassCreateInfo::builder()
                    .attachments(&attachments)
                    .subpasses(&subpasses)
                    .dependencies(&dependencies);

                unsafe {
                    self.device_context.device().create_render_pass(&*create_info, None)
                }
            }
        )
        */
    }

    pub fn load_graphics_pipeline(
        &mut self,
        load_handle: LoadHandle,
        graphics_pipeline: &dsc::GraphicsPipeline,
    ) -> VkResult<vk::Pipeline> {
        let hash = hash_resource_description(&graphics_pipeline);
        if self.graphics_pipelines.contains_resource(hash) {
            Ok(self.graphics_pipelines.add_ref(hash, load_handle, graphics_pipeline))
        } else {
            let pipeline_layout = self.load_pipeline_layout(load_handle, &graphics_pipeline.pipeline_layout)?;
            let renderpass = self.load_renderpass(load_handle, &graphics_pipeline.renderpass)?;
            let resource =
                crate::pipeline_description::create_graphics_pipeline(
                    self.device_context.device(),
                    graphics_pipeline,
                    pipeline_layout,
                    renderpass,
                    &self.swapchain_surface_info
                )?;
            self.graphics_pipelines.insert(hash, load_handle, graphics_pipeline, resource);
            Ok(resource)
        }

        /*
        self.graphics_pipelines.insert_with(
            hash_resource_description(graphics_pipeline),
            load_handle,
            graphics_pipeline,
            || {
                let pipeline_layout = self.load_pipeline_layout(load_handle, &graphics_pipeline.pipeline_layout)?;
                let renderpass = self.load_renderpass(load_handle, &graphics_pipeline.renderpass)?;
                let fixed_function_state = &graphics_pipeline.fixed_function_state;

                let input_assembly_state = fixed_function_state.input_assembly_state.as_builder().build();

                let mut vertex_input_attribute_descriptions: Vec<_> = fixed_function_state.vertex_input_state.attribute_descriptions.iter()
                    .map(|attribute| attribute.as_builder(&self.swapchain_surface_info).build())
                    .collect();

                let mut vertex_input_binding_descriptions: Vec<_> = fixed_function_state.vertex_input_state.binding_descriptions.iter()
                    .map(|binding| binding.as_builder().build())
                    .collect();

                let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
                    .vertex_attribute_descriptions(vertex_input_attribute_descriptions.as_slice())
                    .vertex_binding_descriptions(&vertex_input_binding_descriptions);

                let scissors: Vec<_> = fixed_function_state.viewport_state.scissors.iter()
                    .map(|scissors| scissors.to_rect2d(&self.swapchain_surface_info))
                    .collect();

                let viewports: Vec<_> = fixed_function_state.viewport_state.viewports.iter()
                    .map(|viewport| viewport.as_builder(&self.swapchain_surface_info).build())
                    .collect();

                let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
                    .scissors(&scissors)
                    .viewports(&viewports);

                let rasterization_state = fixed_function_state.rasterization_state.as_builder();

                let multisample_state = fixed_function_state.multisample_state.as_builder();

                let color_blend_attachments: Vec<_> = fixed_function_state.color_blend_state.attachments.iter().map(|attachment| attachment.as_builder().build()).collect();
                let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
                    .logic_op(fixed_function_state.color_blend_state.logic_op.into())
                    .logic_op_enable(fixed_function_state.color_blend_state.logic_op_enable)
                    .blend_constants(fixed_function_state.color_blend_state.blend_constants_as_f32())
                    .attachments(&color_blend_attachments);

                let dynamic_states: Vec<vk::DynamicState> = fixed_function_state.dynamic_state.dynamic_states.iter().map(|dynamic_state| dynamic_state.clone().into()).collect();
                let dynamic_state = PipelineDynamicStateCreateInfo::builder()
                    .dynamic_states(&dynamic_states);


                let mut stages = Vec::with_capacity(graphics_pipeline.pipeline_shader_stages.stages.len());
                let mut shader_modules = Vec::with_capacity(graphics_pipeline.pipeline_shader_stages.stages.len());
                for pipeline_shader_stage in &graphics_pipeline.pipeline_shader_stages.stages {
                    //let module = self.get_or_create_shader_module(&pipeline_shader_stage.shader_module)?;

                    let module = unsafe {
                        let shader_info = vk::ShaderModuleCreateInfo::builder()
                            .code(&pipeline_shader_stage.shader_module.code);
                        self.device_context.device().create_shader_module(&shader_info, None)?
                    };
                    shader_modules.push(module);

                    stages.push(vk::PipelineShaderStageCreateInfo::builder()
                        .stage(pipeline_shader_stage.stage.into())
                        .module(module)
                        .name(&pipeline_shader_stage.entry_name)
                        .build());
                }

                let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
                    .input_assembly_state(&input_assembly_state)
                    .vertex_input_state(&vertex_input_state)
                    .viewport_state(&viewport_state)
                    .rasterization_state(&rasterization_state)
                    .multisample_state(&multisample_state)
                    .color_blend_state(&color_blend_state)
                    .dynamic_state(&dynamic_state)
                    .layout(pipeline_layout)
                    .render_pass(renderpass)
                    .stages(&stages)
                    .build();

                unsafe {
                    match self.device_context.device().create_graphics_pipelines(
                        vk::PipelineCache::null(),
                        &[pipeline_info],
                        None,
                    ) {
                        Ok(result) => Ok(result[0]),
                        Err(e) => Err(e.1),
                    }
                }
            }
        )
        */
    }
}

impl Drop for PipelineManager {
    fn drop(&mut self) {
        unsafe {
            for resource in self.graphics_pipelines.take_all_resources() {
                self.device_context.device().destroy_pipeline(resource, None);
            }

            for resource in self.renderpasses.take_all_resources() {
                self.device_context.device().destroy_render_pass(resource, None);
            }

            for resource in self.pipeline_layouts.take_all_resources() {
                self.device_context.device().destroy_pipeline_layout(resource, None);
            }

            for resource in self.descriptor_set_layouts.take_all_resources() {
                self.device_context.device().destroy_descriptor_set_layout(resource, None);
            }
        }
    }
}