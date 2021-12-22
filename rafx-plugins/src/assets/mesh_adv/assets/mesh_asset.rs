use crate::features::mesh_adv::MeshAdvUntexturedRenderFeatureFlag;
use crate::phases::{DepthPrepassRenderPhase, OpaqueRenderPhase, WireframeRenderPhase};
use crate::shaders::mesh_adv::mesh_adv_textured_frag;
use distill::loader::handle::Handle;
use rafx::api::{RafxIndexType, RafxResult};
use rafx::assets::MaterialInstanceAsset;
use rafx::assets::{
    AssetManager, BufferAsset, DefaultAssetTypeHandler, DefaultAssetTypeLoadHandler,
};
use rafx::framework::render_features::{RenderPhase, RenderPhaseIndex, RenderView};
use rafx::framework::{BufferResource, DescriptorSetArc, MaterialPassResource, ResourceArc};
use rafx::rafx_visibility::VisibleBounds;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use type_uuid::*;

// This is non-texture data associated with the material. Must convert to
// MeshMaterialDataShaderParam to bind to a shader uniform
#[derive(Serialize, Deserialize, Clone)]
#[repr(C)]
pub struct MeshAdvMaterialData {
    // Using f32 arrays for serde support
    pub base_color_factor: [f32; 4],     // default: 1,1,1,1
    pub emissive_factor: [f32; 3],       // default: 0,0,0
    pub metallic_factor: f32,            //default: 1,
    pub roughness_factor: f32,           // default: 1,
    pub normal_texture_scale: f32,       // default: 1
    pub occlusion_texture_strength: f32, // default 1
    pub alpha_cutoff: f32,               // default 0.5

    pub has_base_color_texture: bool,
    pub has_metallic_roughness_texture: bool,
    pub has_normal_texture: bool,
    pub has_occlusion_texture: bool,
    pub has_emissive_texture: bool,
}

impl Default for MeshAdvMaterialData {
    fn default() -> Self {
        MeshAdvMaterialData {
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            emissive_factor: [0.0, 0.0, 0.0],
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            normal_texture_scale: 1.0,
            occlusion_texture_strength: 1.0,
            alpha_cutoff: 0.5,
            has_base_color_texture: false,
            has_metallic_roughness_texture: false,
            has_normal_texture: false,
            has_occlusion_texture: false,
            has_emissive_texture: false,
        }
    }
}

pub type MeshAdvMaterialDataShaderParam = mesh_adv_textured_frag::MaterialDataStd140;

impl Into<MeshAdvMaterialDataShaderParam> for MeshAdvMaterialData {
    fn into(self) -> MeshAdvMaterialDataShaderParam {
        MeshAdvMaterialDataShaderParam {
            base_color_factor: self.base_color_factor.into(),
            emissive_factor: self.emissive_factor.into(),
            metallic_factor: self.metallic_factor,
            roughness_factor: self.roughness_factor,
            normal_texture_scale: self.normal_texture_scale,
            occlusion_texture_strength: self.occlusion_texture_strength,
            alpha_cutoff: self.alpha_cutoff,
            has_base_color_texture: self.has_base_color_texture as u32,
            has_metallic_roughness_texture: self.has_metallic_roughness_texture as u32,
            has_normal_texture: self.has_normal_texture as u32,
            has_occlusion_texture: self.has_occlusion_texture as u32,
            has_emissive_texture: self.has_emissive_texture as u32,
            ..Default::default()
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MeshAdvPartAssetData {
    pub vertex_full_buffer_offset_in_bytes: u32,
    pub vertex_full_buffer_size_in_bytes: u32,
    pub vertex_position_buffer_offset_in_bytes: u32,
    pub vertex_position_buffer_size_in_bytes: u32,
    pub index_buffer_offset_in_bytes: u32,
    pub index_buffer_size_in_bytes: u32,
    pub material_instance: Handle<MaterialInstanceAsset>,
    pub index_type: RafxIndexType,
}

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "4c888448-2650-4f56-82dc-71ba81f4295b"]
pub struct MeshAdvAssetData {
    pub mesh_parts: Vec<MeshAdvPartAssetData>,
    pub vertex_full_buffer: Handle<BufferAsset>, // Vertex type is MeshVertexFull
    pub vertex_position_buffer: Handle<BufferAsset>, // Vertex type is MeshVertexPosition
    pub index_buffer: Handle<BufferAsset>,       // u16 indices
    pub visible_bounds: VisibleBounds,
}

pub struct MeshAdvAssetPart {
    pub material_instance: MaterialInstanceAsset,
    pub textured_pass_index: usize,
    pub untextured_pass_index: usize,
    pub wireframe_pass_index: usize,
    pub vertex_full_buffer_offset_in_bytes: u32,
    pub vertex_full_buffer_size_in_bytes: u32,
    pub vertex_position_buffer_offset_in_bytes: u32,
    pub vertex_position_buffer_size_in_bytes: u32,
    pub index_buffer_offset_in_bytes: u32,
    pub index_buffer_size_in_bytes: u32,
    pub index_type: RafxIndexType,
}

pub const PER_MATERIAL_DESCRIPTOR_SET_LAYOUT_INDEX: usize = 1;

impl MeshAdvAssetPart {
    pub fn get_material_pass_index(
        &self,
        view: &RenderView,
        render_phase_index: RenderPhaseIndex,
    ) -> usize {
        if render_phase_index == OpaqueRenderPhase::render_phase_index() {
            let offset = !view.phase_is_relevant::<DepthPrepassRenderPhase>() as usize;
            return if view.feature_flag_is_relevant::<MeshAdvUntexturedRenderFeatureFlag>() {
                self.untextured_pass_index + offset
            } else {
                self.textured_pass_index + offset
            };
        } else if render_phase_index == WireframeRenderPhase::render_phase_index() {
            self.wireframe_pass_index
        } else {
            panic!(
                "mesh does not support render phase index {}",
                render_phase_index
            )
        }
    }

    pub fn get_material_pass_resource(
        &self,
        view: &RenderView,
        render_phase_index: RenderPhaseIndex,
    ) -> &ResourceArc<MaterialPassResource> {
        &self.material_instance.material.passes
            [self.get_material_pass_index(view, render_phase_index)]
        .material_pass_resource
    }

    pub fn get_material_descriptor_set(
        &self,
        view: &RenderView,
        render_phase_index: RenderPhaseIndex,
    ) -> &DescriptorSetArc {
        return &self.material_instance.material_descriptor_sets
            [self.get_material_pass_index(view, render_phase_index)]
            [PER_MATERIAL_DESCRIPTOR_SET_LAYOUT_INDEX]
            .as_ref()
            .unwrap();
    }
}

pub struct MeshAdvAssetInner {
    pub mesh_parts: Vec<Option<MeshAdvAssetPart>>,
    pub vertex_full_buffer: ResourceArc<BufferResource>,
    pub vertex_position_buffer: ResourceArc<BufferResource>,
    pub index_buffer: ResourceArc<BufferResource>,
    pub asset_data: MeshAdvAssetData,
}

#[derive(TypeUuid, Clone)]
#[uuid = "8a7afe47-8abc-4383-a7c8-0f09026b3019"]
pub struct MeshAdvAsset {
    pub inner: Arc<MeshAdvAssetInner>,
}

pub struct MeshAdvLoadHandler;

impl DefaultAssetTypeLoadHandler<MeshAdvAssetData, MeshAdvAsset> for MeshAdvLoadHandler {
    #[profiling::function]
    fn load(
        asset_manager: &mut AssetManager,
        mesh_asset: MeshAdvAssetData,
    ) -> RafxResult<MeshAdvAsset> {
        let vertex_full_buffer = asset_manager
            .latest_asset(&mesh_asset.vertex_full_buffer)
            .unwrap()
            .buffer
            .clone();
        let vertex_position_buffer = asset_manager
            .latest_asset(&mesh_asset.vertex_position_buffer)
            .unwrap()
            .buffer
            .clone();
        let index_buffer = asset_manager
            .latest_asset(&mesh_asset.index_buffer)
            .unwrap()
            .buffer
            .clone();

        let mesh_parts: Vec<_> = mesh_asset
            .mesh_parts
            .iter()
            .map(|mesh_part| {
                let material_instance = asset_manager
                    .committed_asset(&mesh_part.material_instance)
                    .unwrap();

                let textured_pass_index = material_instance
                    .material
                    .find_pass_by_name("mesh textured")
                    .expect("could not find `mesh textured` pass in mesh part material");

                let textured_z_pass_index = material_instance
                    .material
                    .find_pass_by_name("mesh textured z")
                    .expect("could not find `mesh textured z` pass in mesh part material");

                assert_eq!(
                    textured_z_pass_index,
                    textured_pass_index + 1,
                    "expected `mesh textured z` to occur after `mesh textured`"
                );

                let untextured_pass_index = material_instance
                    .material
                    .find_pass_by_name("mesh untextured")
                    .expect("could not find `mesh untextured` pass in mesh part material");

                let untextured_z_pass_index = material_instance
                    .material
                    .find_pass_by_name("mesh untextured z")
                    .expect("could not find `mesh untextured z` pass in mesh part material");

                assert_eq!(
                    untextured_z_pass_index,
                    untextured_pass_index + 1,
                    "expected `mesh untextured z` to occur after `mesh untextured`"
                );

                let wireframe_pass_index = material_instance
                    .material
                    .find_pass_by_name("mesh wireframe")
                    .expect("could not find `mesh wireframe` pass in mesh part material");

                Some(MeshAdvAssetPart {
                    material_instance: material_instance.clone(),
                    textured_pass_index,
                    untextured_pass_index,
                    wireframe_pass_index,
                    vertex_full_buffer_offset_in_bytes: mesh_part
                        .vertex_full_buffer_offset_in_bytes,
                    vertex_full_buffer_size_in_bytes: mesh_part.vertex_full_buffer_size_in_bytes,
                    vertex_position_buffer_offset_in_bytes: mesh_part
                        .vertex_position_buffer_offset_in_bytes,
                    vertex_position_buffer_size_in_bytes: mesh_part
                        .vertex_position_buffer_size_in_bytes,
                    index_buffer_offset_in_bytes: mesh_part.index_buffer_offset_in_bytes,
                    index_buffer_size_in_bytes: mesh_part.index_buffer_size_in_bytes,
                    index_type: mesh_part.index_type,
                })
            })
            .collect();

        let inner = MeshAdvAssetInner {
            vertex_full_buffer,
            vertex_position_buffer,
            index_buffer,
            asset_data: mesh_asset,
            mesh_parts,
        };

        Ok(MeshAdvAsset {
            inner: Arc::new(inner),
        })
    }
}

pub type MeshAdvAssetType =
    DefaultAssetTypeHandler<MeshAdvAssetData, MeshAdvAsset, MeshAdvLoadHandler>;
