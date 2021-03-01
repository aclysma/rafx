use serde::{Deserialize, Serialize};
use type_uuid::*;

use crate::{AssetManager, ImageAsset, ShaderAsset};
use distill::loader::handle::Handle;
use fnv::FnvHashMap;
use rafx_api::{
    RafxBlendState, RafxBlendStateRenderTarget, RafxCompareOp, RafxCullMode, RafxDepthState,
    RafxError, RafxFillMode, RafxFrontFace, RafxRasterizerState, RafxResult, RafxSamplerDef,
};
pub use rafx_framework::DescriptorSetLayoutResource;
pub use rafx_framework::GraphicsPipelineResource;
use rafx_framework::{
    DescriptorSetArc, FixedFunctionState, MaterialPass, MaterialShaderStage, ResourceArc,
};
use rafx_framework::{DescriptorSetWriteSet, SamplerResource};
use rafx_framework::nodes::{RenderPhase, RenderPhaseIndex};
use std::hash::Hash;
use std::ops::Deref;
use std::sync::Arc;

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
#[uuid = "7f30b29c-7fb9-4b31-a354-7cefbbade2f9"]
pub struct SamplerAssetData {
    pub sampler: RafxSamplerDef,
}

#[derive(TypeUuid, Clone)]
#[uuid = "9fe2825d-a7c5-43f6-97bb-d3385fb2c2c9"]
pub struct SamplerAsset {
    pub sampler: ResourceArc<SamplerResource>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
pub enum AlphaBlendingPreset {
    Disabled,
    Enabled,
}

impl Default for AlphaBlendingPreset {
    fn default() -> Self {
        AlphaBlendingPreset::Disabled
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
pub enum DepthBufferPreset {
    Disabled,
    Enabled,
    EnabledReverseZ,
}

impl Default for DepthBufferPreset {
    fn default() -> Self {
        DepthBufferPreset::Disabled
    }
}

// #[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
// pub enum CullModePreset {
//     UseRasterizerState,
//     Enabled
// }
//
// impl Default for CullModePreset {
//     fn default() -> Self {
//         CullModePreset::UseRasterizerState
//     }
// }
//
// #[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
// pub enum FrontFacePreset {
//     UseRasterizerState,
//     Enabled
// }
//
// impl Default for FrontFacePreset {
//     fn default() -> Self {
//         FrontFacePreset::UseRasterizerState
//     }
// }
//
// #[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
// pub enum FillModePreset {
//     UseRasterizerState,
//     Enabled
// }
//
// impl Default for FillModePreset {
//     fn default() -> Self {
//         FillModePreset::UseRasterizerState
//     }
// }

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq)]
pub struct FixedFunctionStateData {
    #[serde(default)]
    blend_state: RafxBlendState,
    #[serde(default)]
    depth_state: RafxDepthState,
    #[serde(default)]
    rasterizer_state: RafxRasterizerState,

    // These override the above states
    #[serde(default)]
    alpha_blending: AlphaBlendingPreset,
    #[serde(default)]
    depth_testing: DepthBufferPreset,
    #[serde(default)]
    cull_mode: Option<RafxCullMode>,
    #[serde(default)]
    front_face: Option<RafxFrontFace>,
    #[serde(default)]
    fill_mode: Option<RafxFillMode>,
}

impl FixedFunctionStateData {
    pub fn prepare(self) -> RafxResult<FixedFunctionState> {
        let mut blend_state = self.blend_state.clone();
        let mut depth_state = self.depth_state.clone();
        let mut rasterizer_state = self.rasterizer_state.clone();

        match self.alpha_blending {
            AlphaBlendingPreset::Disabled => {
                blend_state.independent_blend = false;
                blend_state.render_target_blend_states =
                    vec![RafxBlendStateRenderTarget::default_alpha_disabled()]
            }
            AlphaBlendingPreset::Enabled => {
                blend_state.independent_blend = false;
                blend_state.render_target_blend_states =
                    vec![RafxBlendStateRenderTarget::default_alpha_enabled()]
            }
        }

        match self.depth_testing {
            DepthBufferPreset::Disabled => {
                depth_state.depth_test_enable = false;
                depth_state.depth_write_enable = false;
            }
            DepthBufferPreset::Enabled => {
                depth_state.depth_test_enable = true;
                depth_state.depth_write_enable = true;
                depth_state.depth_compare_op = RafxCompareOp::LessOrEqual;
            }
            DepthBufferPreset::EnabledReverseZ => {
                depth_state.depth_test_enable = true;
                depth_state.depth_write_enable = true;
                depth_state.depth_compare_op = RafxCompareOp::GreaterOrEqual;
            }
        }

        if let Some(cull_mode) = self.cull_mode {
            rasterizer_state.cull_mode = cull_mode;
        }

        if let Some(fill_mode) = self.fill_mode {
            rasterizer_state.fill_mode = fill_mode;
        }

        if let Some(front_face) = self.front_face {
            rasterizer_state.front_face = front_face;
        }

        Ok(FixedFunctionState {
            blend_state,
            depth_state,
            rasterizer_state,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GraphicsPipelineShaderStage {
    pub stage: MaterialShaderStage,
    pub shader_module: Handle<ShaderAsset>,
    pub entry_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MaterialPassData {
    pub name: Option<String>,
    pub phase: Option<String>,
    pub fixed_function_state: FixedFunctionStateData,
    pub shaders: Vec<GraphicsPipelineShaderStage>,
}

impl MaterialPassData {
    #[profiling::function]
    pub fn create_material_pass(
        &self,
        asset_manager: &AssetManager,
    ) -> RafxResult<MaterialPass> {
        use distill::loader::handle::AssetHandle;

        //
        // Gather shader stage info
        //
        let mut shader_modules = Vec::with_capacity(self.shaders.len());
        let mut entry_points = Vec::with_capacity(self.shaders.len());

        // We iterate through the entry points we will hit for each stage. Each stage may define
        // slightly different reflection data/bindings in use.
        for stage in &self.shaders {
            log::trace!(
                "Set up material pass stage: {:?} material pass name: {:?}",
                stage,
                self.name
            );

            let shader_asset = asset_manager
                .loaded_assets()
                .shader_modules
                .get_latest(stage.shader_module.load_handle())
                .unwrap();
            shader_modules.push(shader_asset.shader_module.clone());

            let reflection_data = shader_asset.reflection_data.get(&stage.entry_name);
            let reflection_data = reflection_data.ok_or_else(|| {
                let error_message = format!(
                    "Load Material Failed - Pass refers to entry point named {}, but no matching reflection data was found",
                    stage.entry_name
                );
                log::error!("{}", error_message);
                error_message
            })?;

            entry_points.push(reflection_data);

            // Check that the compiled shader supports the given stage
            if (reflection_data.rafx_api_reflection.shader_stage & stage.stage.into()).is_empty() {
                let error = format!(
                    "Load Material Failed - Pass is using a shader for stage {:?}, but this shader supports stages {:?}.",
                    stage.stage,
                    reflection_data.rafx_api_reflection.shader_stage
                );
                log::error!("{}", error);
                return Err(error)?;
            }

            log::trace!("  Reflection data:\n{:#?}", reflection_data);
        }

        let fixed_function_state = Arc::new(self.fixed_function_state.clone().prepare()?);

        //
        // We now have everything needed to create the framework-level material pass
        //
        let resource_context = asset_manager.resource_manager().resource_context();
        let material_pass = MaterialPass::new(
            &resource_context,
            fixed_function_state,
            shader_modules,
            &entry_points,
        )
        .map_err(|x| {
            RafxError::StringError(format!(
                "While loading pass '{:?}' for phase '{:?}': {:?}",
                self.name, self.phase, x
            ))
        })?;

        //
        // If a phase name is specified, register the pass with the pipeline cache. The pipeline
        // cache is responsible for ensuring pipelines are created for renderpasses that execute
        // within the pipeline's phase
        //
        if let Some(phase_name) = &self.phase {
            let render_phase_index = resource_context
                .graphics_pipeline_cache()
                .get_render_phase_by_name(phase_name);
            match render_phase_index {
                Some(render_phase_index) => resource_context
                    .graphics_pipeline_cache()
                    .register_material_to_phase_index(
                        &material_pass.material_pass_resource,
                        render_phase_index,
                    ),
                None => {
                    let error = format!(
                        "Load Material Failed - Pass refers to phase name {}, but this phase name was not registered",
                        phase_name
                    );
                    log::error!("{}", error);
                    return Err(error)?;
                }
            }

            render_phase_index
        } else {
            None
        };

        Ok(material_pass)
    }
}

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[uuid = "ad94bca2-1f02-4e5f-9117-1a7b03456a11"]
pub struct MaterialAssetData {
    pub passes: Vec<MaterialPassData>,
}

pub struct MaterialAssetInner {
    //TODO: Consider making this named
    //TODO: Get cached graphics pipelines working
    //TODO: Could consider decoupling render cache from phases
    pub passes: Vec<MaterialPass>,
    pub pass_name_to_index: FnvHashMap<String, usize>,
    pub pass_phase_to_index: FnvHashMap<RenderPhaseIndex, usize>,
}

#[derive(TypeUuid, Clone)]
#[uuid = "165673cd-d81d-4708-b9a4-d7e1a2a67976"]
pub struct MaterialAsset {
    pub inner: Arc<MaterialAssetInner>,
}

impl MaterialAsset {
    pub fn new(
        passes: Vec<MaterialPass>,
        pass_name_to_index: FnvHashMap<String, usize>,
        pass_phase_to_index: FnvHashMap<RenderPhaseIndex, usize>,
    ) -> Self {
        let inner = MaterialAssetInner {
            passes,
            pass_name_to_index,
            pass_phase_to_index,
        };

        MaterialAsset {
            inner: Arc::new(inner),
        }
    }

    pub fn find_pass_by_name(
        &self,
        name: &str,
    ) -> Option<usize> {
        self.inner.pass_name_to_index.get(name).copied()
    }

    pub fn find_pass_by_phase<T: RenderPhase>(&self) -> Option<usize> {
        self.inner
            .pass_phase_to_index
            .get(&T::render_phase_index())
            .copied()
    }

    pub fn find_pass_by_phase_index(
        &self,
        index: RenderPhaseIndex,
    ) -> Option<usize> {
        self.inner.pass_phase_to_index.get(&index).copied()
    }
}

impl Deref for MaterialAsset {
    type Target = MaterialAssetInner;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MaterialInstanceSlotAssignment {
    pub slot_name: String,
    pub image: Option<Handle<ImageAsset>>,
    pub sampler: Option<RafxSamplerDef>,

    // Would be nice to use this, but I don't think it works with Option
    //#[serde(with = "serde_bytes")]
    pub buffer_data: Option<Vec<u8>>,
}

#[derive(TypeUuid, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[uuid = "0d8cacf7-79df-4aa6-b99e-659a9c3b5e6b"]
pub struct MaterialInstanceAssetData {
    pub material: Handle<MaterialAsset>,
    pub slot_assignments: Vec<MaterialInstanceSlotAssignment>,
}

pub struct MaterialInstanceAssetInner {
    pub material_handle: Handle<MaterialAsset>,
    pub material: MaterialAsset,

    // Arc these individually because some downstream systems care only about the descriptor sets
    pub material_descriptor_sets: Arc<Vec<Vec<Option<DescriptorSetArc>>>>,
    pub slot_assignments: Vec<MaterialInstanceSlotAssignment>,
    pub descriptor_set_writes: Vec<Vec<DescriptorSetWriteSet>>,
}

#[derive(TypeUuid, Clone)]
#[uuid = "c60f6a3d-3e8d-4eea-8576-0971cd71b60f"]
pub struct MaterialInstanceAsset {
    pub inner: Arc<MaterialInstanceAssetInner>,
}

impl MaterialInstanceAsset {
    pub fn new(
        material: Handle<MaterialAsset>,
        material_asset: MaterialAsset,
        material_descriptor_sets: Arc<Vec<Vec<Option<DescriptorSetArc>>>>,
        slot_assignments: Vec<MaterialInstanceSlotAssignment>,
        descriptor_set_writes: Vec<Vec<DescriptorSetWriteSet>>,
    ) -> Self {
        let inner = MaterialInstanceAssetInner {
            material_handle: material,
            material: material_asset,
            material_descriptor_sets,
            slot_assignments,
            descriptor_set_writes,
        };

        MaterialInstanceAsset {
            inner: Arc::new(inner),
        }
    }
}

impl Deref for MaterialInstanceAsset {
    type Target = MaterialInstanceAssetInner;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}
