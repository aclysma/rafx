use crate::assets::mesh_adv::{MeshAdvBufferAsset, MeshMaterialAdvAsset};
use crate::features::mesh_adv::MeshAdvUntexturedRenderFeatureFlag;
use crate::phases::{OpaqueRenderPhase, TransparentRenderPhase, WireframeRenderPhase};
use hydrate_base::handle::AssetHandle;
use hydrate_base::{Handle, LoadHandle};
use rafx::api::{RafxIndexType, RafxResult};
use rafx::assets::{
    AssetManager, DefaultAssetTypeHandler, DefaultAssetTypeLoadHandler, MaterialAsset,
};
use rafx::framework::render_features::{RenderPhase, RenderPhaseIndex, RenderView};
use rafx::framework::{MaterialPassResource, ResourceArc};
use rafx::rafx_visibility::VisibleBounds;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use type_uuid::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeshAdvShadowMethod {
    None,
    Opaque,
    //AlphaClip,
    //AlphaStochastic,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeshAdvBlendMethod {
    Opaque,
    AlphaClip,
    //AlphaStochastic,
    AlphaBlend,
}

// This is non-texture data associated with the material. Must convert to
// MeshMaterialDataShaderParam to bind to a shader uniform
#[derive(Serialize, Deserialize, Clone, Debug, TypeUuid)]
#[repr(C)]
#[uuid = "e7c4f03b-8c1a-4fbc-9f98-83e521687777"]
pub struct MeshAdvMaterialData {
    // Using f32 arrays for serde support
    pub base_color_factor: [f32; 4], // default: 1,1,1,1
    pub emissive_factor: [f32; 3],   // default: 0,0,0
    pub metallic_factor: f32,        //default: 1,
    pub roughness_factor: f32,       // default: 1,
    pub normal_texture_scale: f32,   // default: 1

    pub has_base_color_texture: bool,
    pub base_color_texture_has_alpha_channel: bool,
    pub has_metallic_roughness_texture: bool,
    pub has_normal_texture: bool,
    pub has_emissive_texture: bool,

    pub shadow_method: MeshAdvShadowMethod,
    pub blend_method: MeshAdvBlendMethod,
    pub alpha_threshold: f32,
    pub backface_culling: bool,
}

impl Default for MeshAdvMaterialData {
    fn default() -> Self {
        MeshAdvMaterialData {
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            emissive_factor: [0.0, 0.0, 0.0],
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            normal_texture_scale: 1.0,
            has_base_color_texture: false,
            base_color_texture_has_alpha_channel: false,
            has_metallic_roughness_texture: false,
            has_normal_texture: false,
            has_emissive_texture: false,
            shadow_method: MeshAdvShadowMethod::Opaque,
            blend_method: MeshAdvBlendMethod::Opaque,
            alpha_threshold: 0.5,
            backface_culling: true,
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
    pub mesh_material: Handle<MeshMaterialAdvAsset>,
    pub index_type: RafxIndexType,
}

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "4c888448-2650-4f56-82dc-71ba81f4295b"]
pub struct MeshAdvAssetData {
    pub mesh_parts: Vec<MeshAdvPartAssetData>,
    pub vertex_full_buffer: Handle<MeshAdvBufferAsset>, // Vertex type is MeshVertexFull
    pub vertex_position_buffer: Handle<MeshAdvBufferAsset>, // Vertex type is MeshVertexPosition
    pub index_buffer: Handle<MeshAdvBufferAsset>,       // u16 indices
    pub visible_bounds: VisibleBounds,
}

#[derive(Clone)]
pub struct MeshAdvShaderPassIndices {
    //pub depth_prepass: u8,
    ////pub depth_prepass_backface: u8,
    ////pub depth_prepass_velocity: u8,
    ////pub depth_prepass_velocity_backface: u8,
    //pub depth_prepass_velocity_moved: u8,
    ////pub depth_prepass_velocity_backface_moved: u8,
    //

    // For shadow maps we don't do any face culling
    pub shadow_map: u8,
    //pub shadow_map_backface: u8,
    pub opaque: u8,
    pub opaque_backface: u8,
    //pub opaque_alphaclip: u8,
    //pub opaque_alphaclip_backface: u8,
    pub opaque_untextured: u8,
    pub opaque_backface_untextured: u8,
    //pub opaque_alphaclip_untextured: u8,
    //pub opaque_alphaclip_backface_untextured: u8,
    pub transparent: u8,
    pub transparent_backface: u8,
    pub transparent_untextured: u8,
    pub transparent_backface_untextured: u8,

    pub wireframe: u8,
}

impl MeshAdvShaderPassIndices {
    #[rustfmt::skip]
    pub fn new(material: &MaterialAsset) -> MeshAdvShaderPassIndices {
        //let depth_prepass = material.find_pass_index_by_name("depth_prepass").expect("Mesh shader must have pass named 'depth_prepass'") as u8;
        ////let depth_prepass_backface = material.find_pass_index_by_name("depth_prepass_backface").expect("Mesh shader must have pass named 'depth_prepass_backface'") as u8;
        ////let depth_prepass_velocity = material.find_pass_index_by_name("depth_prepass_velocity").expect("Mesh shader must have pass named 'depth_prepass_velocity'") as u8;
        ////let depth_prepass_velocity_backface = material.find_pass_index_by_name("depth_prepass_velocity_backface").expect("Mesh shader must have pass named 'depth_prepass_velocity_backface'") as u8;
        //let depth_prepass_velocity_moved = material.find_pass_index_by_name("depth_prepass_velocity_moved").expect("Mesh shader must have pass named 'depth_prepass_velocity_moved'") as u8;
        ////let depth_prepass_velocity_backface_moved = material.find_pass_index_by_name("depth_prepass_velocity_backface_moved").expect("Mesh shader must have pass named 'depth_prepass_velocity_backface_moved'") as u8;
        let shadow_map = material.find_pass_index_by_name("shadow_map").expect("Mesh shader must have pass named 'shadow_map'") as u8;
        //let shadow_map_backface = material.find_pass_index_by_name("shadow_map_backface").expect("Mesh shader must have pass named 'shadow_map_backface'") as u8;
        let opaque = material.find_pass_index_by_name("opaque").expect("Mesh shader must have pass named 'opaque'") as u8;
        let opaque_backface = material.find_pass_index_by_name("opaque_backface").expect("Mesh shader must have pass named 'opaque_backface'") as u8;
        //let opaque_alphaclip = material.find_pass_index_by_name("opaque_alphaclip").expect("Mesh shader must have pass named 'opaque_alphaclip'") as u8;
        //let opaque_alphaclip_backface = material.find_pass_index_by_name("opaque_alphaclip_backface").expect("Mesh shader must have pass named 'opaque_alphaclip_backface'") as u8;
        let opaque_untextured = material.find_pass_index_by_name("opaque_untextured").expect("Mesh shader must have pass named 'opaque'") as u8;
        let opaque_backface_untextured = material.find_pass_index_by_name("opaque_backface_untextured").expect("Mesh shader must have pass named 'opaque_backface'") as u8;
        //let opaque_alphaclip_untextured = material.find_pass_index_by_name("opaque_alphaclip").expect("Mesh shader must have pass named 'opaque_alphaclip'") as u8;
        //let opaque_alphaclip_backface_untextured = material.find_pass_index_by_name("opaque_alphaclip_backface").expect("Mesh shader must have pass named 'opaque_alphaclip_backface'") as u8;
        let transparent = material.find_pass_index_by_name("transparent").expect("Mesh shader must have pass named 'transparent'") as u8;
        let transparent_backface = material.find_pass_index_by_name("transparent_backface").expect("Mesh shader must have pass named 'transparent_backface'") as u8;
        let transparent_untextured = material.find_pass_index_by_name("transparent_untextured").expect("Mesh shader must have pass named 'transparent'") as u8;
        let transparent_backface_untextured = material.find_pass_index_by_name("transparent_backface_untextured").expect("Mesh shader must have pass named 'transparent_backface'") as u8;
        let wireframe = material.find_pass_index_by_name("wireframe").expect("Mesh shader must have pass named 'wireframe'") as u8;

        MeshAdvShaderPassIndices {
            //depth_prepass,
            ////depth_prepass_backface,
            ////depth_prepass_velocity,
            ////depth_prepass_velocity_backface,
            //depth_prepass_velocity_moved,
            ////depth_prepass_velocity_backface_moved,
            shadow_map,
            //shadow_map_backface,
            opaque,
            opaque_backface,
            //opaque_alphaclip,
            //opaque_alphaclip_backface,
            opaque_untextured,
            opaque_backface_untextured,
            //opaque_alphaclip_untextured,
            //opaque_alphaclip_backface_untextured,
            transparent,
            transparent_backface,
            transparent_untextured,
            transparent_backface_untextured,
            wireframe
        }
    }

    pub fn get_material_pass_index(
        &self,
        material_data: &MeshAdvMaterialData,
        render_phase_index: RenderPhaseIndex,
        untextured: bool,
    ) -> usize {
        let pass_index = if render_phase_index == OpaqueRenderPhase::render_phase_index() {
            if material_data.backface_culling {
                if untextured {
                    self.opaque_untextured
                } else {
                    self.opaque
                }
            } else {
                if untextured {
                    self.opaque_backface_untextured
                } else {
                    self.opaque_backface
                }
            }
        } else if render_phase_index == TransparentRenderPhase::render_phase_index() {
            if material_data.backface_culling {
                if untextured {
                    self.transparent_untextured
                } else {
                    self.transparent
                }
            } else {
                if untextured {
                    self.transparent_backface_untextured
                } else {
                    self.transparent_backface
                }
            }
        } else if render_phase_index == WireframeRenderPhase::render_phase_index() {
            self.wireframe
        } else {
            panic!(
                "mesh does not support render phase index {}",
                render_phase_index
            )
        };

        pass_index as usize
    }
}

pub struct MeshAdvAssetPart {
    pub mesh_material: MeshMaterialAdvAsset,
    pub pass_indices: MeshAdvShaderPassIndices,
    pub vertex_full_buffer_offset_in_bytes: u32,
    pub vertex_full_buffer_size_in_bytes: u32,
    pub vertex_position_buffer_offset_in_bytes: u32,
    pub vertex_position_buffer_size_in_bytes: u32,
    pub index_buffer_offset_in_bytes: u32,
    pub index_buffer_size_in_bytes: u32,
    pub index_type: RafxIndexType,
}

impl MeshAdvAssetPart {
    fn get_material_pass_index(
        &self,
        view: &RenderView,
        render_phase_index: RenderPhaseIndex,
    ) -> usize {
        let untextured = view.feature_flag_is_relevant::<MeshAdvUntexturedRenderFeatureFlag>();
        self.pass_indices.get_material_pass_index(
            self.mesh_material.material_data(),
            render_phase_index,
            untextured,
        )
    }

    pub fn get_material_pass_resource(
        &self,
        view: &RenderView,
        render_phase_index: RenderPhaseIndex,
    ) -> &ResourceArc<MaterialPassResource> {
        &self.mesh_material.material_asset().passes
            [self.get_material_pass_index(view, render_phase_index)]
        .material_pass_resource
    }
}

pub struct MeshAdvAssetInner {
    pub mesh_parts: Vec<MeshAdvAssetPart>,
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
        _load_handle: LoadHandle,
    ) -> RafxResult<MeshAdvAsset> {
        let vertex_full_buffer_byte_offset = asset_manager
            .latest_asset(&mesh_asset.vertex_full_buffer)
            .unwrap()
            .buffer_byte_offset();
        let vertex_position_buffer_byte_offset = asset_manager
            .latest_asset(&mesh_asset.vertex_position_buffer)
            .unwrap()
            .buffer_byte_offset();
        let index_buffer_byte_offset = asset_manager
            .latest_asset(&mesh_asset.index_buffer)
            .unwrap()
            .buffer_byte_offset();

        let mesh_parts: Vec<_> = mesh_asset
            .mesh_parts
            .iter()
            .map(|mesh_part| {
                let mesh_material = asset_manager
                    .latest_asset(&mesh_part.mesh_material)
                    .unwrap();

                let material_asset = mesh_material.material_asset();
                let pass_indices = MeshAdvShaderPassIndices::new(&material_asset);

                MeshAdvAssetPart {
                    mesh_material: mesh_material.clone(),
                    pass_indices,
                    vertex_full_buffer_offset_in_bytes: vertex_full_buffer_byte_offset
                        + mesh_part.vertex_full_buffer_offset_in_bytes,
                    vertex_full_buffer_size_in_bytes: mesh_part.vertex_full_buffer_size_in_bytes,
                    vertex_position_buffer_offset_in_bytes: vertex_position_buffer_byte_offset
                        + mesh_part.vertex_position_buffer_offset_in_bytes,
                    vertex_position_buffer_size_in_bytes: mesh_part
                        .vertex_position_buffer_size_in_bytes,
                    index_buffer_offset_in_bytes: index_buffer_byte_offset
                        + mesh_part.index_buffer_offset_in_bytes,
                    index_buffer_size_in_bytes: mesh_part.index_buffer_size_in_bytes,
                    index_type: mesh_part.index_type,
                }
            })
            .collect();

        let inner = MeshAdvAssetInner {
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
