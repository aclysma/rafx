use serde::{Deserialize, Serialize};
use type_uuid::*;

use crate::{AssetManager, DefaultAssetTypeHandler, DefaultAssetTypeLoadHandler, ShaderAsset};
use distill::loader::handle::Handle;
use rafx_api::RafxResult;
pub use rafx_framework::DescriptorSetLayoutResource;
pub use rafx_framework::GraphicsPipelineResource;
use rafx_framework::{ComputePipelineResource, DescriptorSetLayout, ResourceArc};
use std::hash::Hash;

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
#[uuid = "e70aa3d2-5727-433a-80c2-4f6f1d01c91f"]
pub struct ComputePipelineAssetData {
    pub shader_module: Handle<ShaderAsset>,
    pub entry_name: String,
}

// The actual GPU resources are held in Material because the pipeline does not specify everything
// needed to create the pipeline
#[derive(TypeUuid, Clone)]
#[uuid = "d5673f07-c926-4e75-bab9-4e8b64e87f22"]
pub struct ComputePipelineAsset {
    // We need to keep a copy of the asset so that we can recreate the pipeline for new swapchains
    pub compute_pipeline: ResourceArc<ComputePipelineResource>,
}

pub struct ComputePipelineLoadHandler;

impl DefaultAssetTypeLoadHandler<ComputePipelineAssetData, ComputePipelineAsset>
    for ComputePipelineLoadHandler
{
    #[profiling::function]
    fn load(
        asset_manager: &mut AssetManager,
        asset_data: ComputePipelineAssetData,
    ) -> RafxResult<ComputePipelineAsset> {
        //
        // Get the shader module
        //
        let shader_module = asset_manager
            .latest_asset(&asset_data.shader_module)
            .unwrap();

        //
        // Find the reflection data in the shader module for the given entry point
        //
        let reflection_data = shader_module.reflection_data.get(&asset_data.entry_name);
        let reflection_data = reflection_data.ok_or_else(|| {
            let error_message = format!(
                "Load Compute Shader Failed - Pass refers to entry point named {}, but no matching reflection data was found",
                asset_data.entry_name
            );
            log::error!("{}", error_message);
            error_message
        })?;

        let shader = asset_manager
            .resources()
            .get_or_create_shader(&[shader_module.shader_module.clone()], &[&reflection_data])?;

        let root_signature =
            asset_manager
                .resources()
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
            let descriptor_set_layout = asset_manager
                .resources()
                .get_or_create_descriptor_set_layout(
                    &root_signature,
                    set_index as u32,
                    &descriptor_set_layout_def,
                )?;
            descriptor_set_layouts.push(descriptor_set_layout);
        }

        //
        // Create the compute pipeline
        //
        let compute_pipeline = asset_manager.resources().get_or_create_compute_pipeline(
            &shader,
            &root_signature,
            descriptor_set_layouts,
        )?;

        Ok(ComputePipelineAsset { compute_pipeline })
    }
}

pub type ComputePipelineAssetTypeHandler = DefaultAssetTypeHandler<
    ComputePipelineAssetData,
    ComputePipelineAsset,
    ComputePipelineLoadHandler,
>;
