use crate::vk_description as dsc;
use super::ResourceHash;
use crate::{ResourceArc, DescriptorSetLayoutResource, PipelineLayoutResource, ResourceManager, PipelineAssetData, RenderpassAssetData, MaterialPassData};
use ash::vk;
use ash::prelude::VkResult;
use atelier_assets::loader::handle::AssetHandle;

// We have to create pipelines when pipeline assets load and when swapchains are added/removed.
// Gathering all the info to hash and create a pipeline is a bit involved so we share the code
// here
#[derive(Clone)]
pub struct PipelineCreateData {
    // We store the shader module hash rather than the shader module itself because it's a large
    // binary blob. We don't use it to create or even look up the shader, just so that the hash of
    // the pipeline we create doesn't conflict with the same pipeline using reloaded (different) shaders
    shader_module_metas: Vec<dsc::ShaderModuleMeta>,
    shader_module_hashes: Vec<ResourceHash>,
    shader_module_arcs: Vec<ResourceArc<vk::ShaderModule>>,
    shader_module_vk_objs: Vec<vk::ShaderModule>,

    descriptor_set_layout_arcs: Vec<ResourceArc<DescriptorSetLayoutResource>>,

    fixed_function_state: dsc::FixedFunctionState,

    pipeline_layout_def: dsc::PipelineLayout,
    pipeline_layout: ResourceArc<PipelineLayoutResource>,

    renderpass: dsc::RenderPass,
}

impl PipelineCreateData {
    pub fn shader_module_metas(&self) -> &Vec<dsc::ShaderModuleMeta> {
        &self.shader_module_metas
    }

    pub fn shader_module_hashes(&self) -> &Vec<ResourceHash> {
        &self.shader_module_hashes
    }

    pub fn shader_module_arcs(&self) -> &Vec<ResourceArc<vk::ShaderModule>> {
        &self.shader_module_arcs
    }

    pub fn shader_module_vk_objs(&self) -> &Vec<vk::ShaderModule> {
        &self.shader_module_vk_objs
    }

    pub fn descriptor_set_layout_arcs(&self) -> &Vec<ResourceArc<DescriptorSetLayoutResource>> {
        &self.descriptor_set_layout_arcs
    }

    pub fn fixed_function_state(&self) -> &dsc::FixedFunctionState {
        &self.fixed_function_state
    }

    pub fn pipeline_layout_def(&self) -> &dsc::PipelineLayout {
        &self.pipeline_layout_def
    }

    pub fn pipeline_layout(&self) -> &ResourceArc<PipelineLayoutResource> {
        &self.pipeline_layout
    }

    pub fn renderpass_def(&self) -> &dsc::RenderPass {
        &self.renderpass
    }

    pub fn new(
        resource_manager: &mut ResourceManager,
        pipeline_asset: &PipelineAssetData,
        renderpass_asset: &RenderpassAssetData,
        material_pass: &MaterialPassData,
        shader_module_hashes: Vec<ResourceHash>,
    ) -> VkResult<Self> {
        //
        // Shader module metadata (required to create the pipeline key)
        //
        let mut shader_module_metas = Vec::with_capacity(material_pass.shaders.len());
        for stage in &material_pass.shaders {
            let shader_module_meta = dsc::ShaderModuleMeta {
                stage: stage.stage,
                entry_name: stage.entry_name.clone(),
            };
            shader_module_metas.push(shader_module_meta);
        }

        //
        // Actual shader module resources (to create the pipeline)
        //
        let mut shader_module_arcs = Vec::with_capacity(material_pass.shaders.len());
        let mut shader_module_vk_objs = Vec::with_capacity(material_pass.shaders.len());
        for stage in &material_pass.shaders {
            let shader_module = resource_manager
                .loaded_assets()
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
        let mut descriptor_set_layout_defs =
            Vec::with_capacity(material_pass.shader_interface.descriptor_set_layouts.len());
        for descriptor_set_layout_def in &material_pass.shader_interface.descriptor_set_layouts {
            let descriptor_set_layout_def = descriptor_set_layout_def.into();
            let descriptor_set_layout = resource_manager
                .resources_mut()
                .get_or_create_descriptor_set_layout(&descriptor_set_layout_def)?;
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

        let pipeline_layout = resource_manager
            .resources_mut()
            .get_or_create_pipeline_layout(&pipeline_layout_def)?;

        let fixed_function_state = dsc::FixedFunctionState {
            vertex_input_state: material_pass.shader_interface.vertex_input_state.clone(),
            input_assembly_state: pipeline_asset.input_assembly_state.clone(),
            viewport_state: pipeline_asset.viewport_state.clone(),
            rasterization_state: pipeline_asset.rasterization_state.clone(),
            multisample_state: pipeline_asset.multisample_state.clone(),
            color_blend_state: pipeline_asset.color_blend_state.clone(),
            dynamic_state: pipeline_asset.dynamic_state.clone(),
            depth_stencil_state: pipeline_asset.depth_stencil_state.clone(),
        };

        Ok(PipelineCreateData {
            shader_module_metas,
            shader_module_hashes,
            shader_module_arcs,
            shader_module_vk_objs,
            descriptor_set_layout_arcs,
            fixed_function_state,
            pipeline_layout_def,
            pipeline_layout,
            renderpass: renderpass_asset.renderpass.clone(),
        })
    }
}