use crate::assets::mesh_basic::MeshMaterialBasicAsset;
use crate::features::mesh_basic::MeshBasicUntexturedRenderFeatureFlag;
use crate::phases::{OpaqueRenderPhase, TransparentRenderPhase, WireframeRenderPhase};
use crate::shaders::mesh_basic::mesh_basic_textured_frag;
use distill::loader::handle::{ArtifactHandle, Handle};
use hydrate_base::LoadHandle;
use rafx::api::{RafxIndexType, RafxResult};
use rafx::assets::{
    AssetManager, BufferAsset, DefaultAssetTypeHandler, DefaultAssetTypeLoadHandler, MaterialAsset,
};
use rafx::framework::render_features::{RenderPhase, RenderPhaseIndex, RenderView};
use rafx::framework::{BufferResource, DescriptorSetArc, MaterialPassResource, ResourceArc};
use rafx::rafx_visibility::VisibleBounds;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use type_uuid::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeshBasicShadowMethod {
    None,
    Opaque,
    //AlphaClip,
    //AlphaStochastic,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeshBasicBlendMethod {
    Opaque,
    AlphaClip,
    //AlphaStochastic,
    AlphaBlend,
}

// This is non-texture data associated with the material. Must convert to
// MeshMaterialDataShaderParam to bind to a shader uniform
#[derive(Serialize, Deserialize, Clone)]
#[repr(C)]
pub struct MeshBasicMaterialData {
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

    pub shadow_method: MeshBasicShadowMethod,
    pub blend_method: MeshBasicBlendMethod,
    pub alpha_threshold: f32,
    pub backface_culling: bool,
}

impl Default for MeshBasicMaterialData {
    fn default() -> Self {
        MeshBasicMaterialData {
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
            shadow_method: MeshBasicShadowMethod::Opaque,
            blend_method: MeshBasicBlendMethod::Opaque,
            alpha_threshold: 0.5,
            backface_culling: true,
        }
    }
}

pub type MeshBasicMaterialDataShaderParam = mesh_basic_textured_frag::MaterialDataStd140;

impl Into<MeshBasicMaterialDataShaderParam> for MeshBasicMaterialData {
    fn into(self) -> MeshBasicMaterialDataShaderParam {
        MeshBasicMaterialDataShaderParam {
            base_color_factor: self.base_color_factor.into(),
            emissive_factor: self.emissive_factor.into(),
            metallic_factor: self.metallic_factor,
            roughness_factor: self.roughness_factor,
            normal_texture_scale: self.normal_texture_scale,
            alpha_threshold: self.alpha_threshold,
            enable_alpha_blend: (self.blend_method == MeshBasicBlendMethod::AlphaBlend) as u32,
            enable_alpha_clip: (self.blend_method == MeshBasicBlendMethod::AlphaClip) as u32,
            has_base_color_texture: self.has_base_color_texture as u32,
            base_color_texture_has_alpha_channel: self.base_color_texture_has_alpha_channel as u32,
            has_metallic_roughness_texture: self.has_metallic_roughness_texture as u32,
            has_normal_texture: self.has_normal_texture as u32,
            has_emissive_texture: self.has_emissive_texture as u32,
            _padding0: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MeshBasicPartAssetData {
    pub vertex_full_buffer_offset_in_bytes: u32,
    pub vertex_full_buffer_size_in_bytes: u32,
    pub vertex_position_buffer_offset_in_bytes: u32,
    pub vertex_position_buffer_size_in_bytes: u32,
    pub index_buffer_offset_in_bytes: u32,
    pub index_buffer_size_in_bytes: u32,
    pub mesh_material: Handle<MeshMaterialBasicAsset>,
    pub index_type: RafxIndexType,
}

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "cf232526-3757-4d94-98d1-c2f7e27c979f"]
pub struct MeshBasicAssetData {
    pub mesh_parts: Vec<MeshBasicPartAssetData>,
    pub vertex_full_buffer: Handle<BufferAsset>, // Vertex type is MeshVertexFull
    pub vertex_position_buffer: Handle<BufferAsset>, // Vertex type is MeshVertexPosition
    pub index_buffer: Handle<BufferAsset>,       // u16 indices
    pub visible_bounds: VisibleBounds,
}

#[derive(Clone)]
pub struct MeshBasicShaderPassIndices {
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

impl MeshBasicShaderPassIndices {
    #[rustfmt::skip]
    pub fn new(material: &MaterialAsset) -> MeshBasicShaderPassIndices {
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

        MeshBasicShaderPassIndices {
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
        material_data: &MeshBasicMaterialData,
        view: &RenderView,
        render_phase_index: RenderPhaseIndex,
    ) -> usize {
        let untextured = view.feature_flag_is_relevant::<MeshBasicUntexturedRenderFeatureFlag>();
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

pub struct MeshBasicAssetPart {
    pub mesh_material: MeshMaterialBasicAsset,
    pub pass_indices: MeshBasicShaderPassIndices,
    pub vertex_full_buffer_offset_in_bytes: u32,
    pub vertex_full_buffer_size_in_bytes: u32,
    pub vertex_position_buffer_offset_in_bytes: u32,
    pub vertex_position_buffer_size_in_bytes: u32,
    pub index_buffer_offset_in_bytes: u32,
    pub index_buffer_size_in_bytes: u32,
    pub index_type: RafxIndexType,
}

pub const PER_MATERIAL_DESCRIPTOR_SET_LAYOUT_INDEX: usize =
    mesh_basic_textured_frag::PER_MATERIAL_DATA_DESCRIPTOR_SET_INDEX;

impl MeshBasicAssetPart {
    pub fn get_material_pass_resource(
        &self,
        view: &RenderView,
        render_phase_index: RenderPhaseIndex,
    ) -> &ResourceArc<MaterialPassResource> {
        &self.mesh_material.material_instance().material.passes[self
            .pass_indices
            .get_material_pass_index(
                &self.mesh_material.data().material_data,
                view,
                render_phase_index,
            )]
        .material_pass_resource
    }

    pub fn get_material_descriptor_set(
        &self,
        view: &RenderView,
        render_phase_index: RenderPhaseIndex,
    ) -> &DescriptorSetArc {
        return &self
            .mesh_material
            .material_instance()
            .material_descriptor_sets[self.pass_indices.get_material_pass_index(
            &self.mesh_material.data().material_data,
            view,
            render_phase_index,
        )][PER_MATERIAL_DESCRIPTOR_SET_LAYOUT_INDEX]
            .as_ref()
            .unwrap();
    }
}

pub struct MeshBasicAssetInner {
    pub mesh_parts: Vec<Option<MeshBasicAssetPart>>,
    pub vertex_full_buffer: ResourceArc<BufferResource>,
    pub vertex_position_buffer: ResourceArc<BufferResource>,
    pub index_buffer: ResourceArc<BufferResource>,
    pub asset_data: MeshBasicAssetData,
}

#[derive(TypeUuid, Clone)]
#[uuid = "689a0bf0-e320-41c0-b4e8-bdb2055a7a57"]
pub struct MeshBasicAsset {
    pub inner: Arc<MeshBasicAssetInner>,
}

pub struct MeshBasicLoadHandler;

impl DefaultAssetTypeLoadHandler<MeshBasicAssetData, MeshBasicAsset> for MeshBasicLoadHandler {
    #[profiling::function]
    fn load(
        asset_manager: &mut AssetManager,
        mesh_asset: MeshBasicAssetData,
        _load_handle: LoadHandle,
    ) -> RafxResult<MeshBasicAsset> {
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
                let mesh_material = asset_manager
                    .latest_asset(&mesh_part.mesh_material)
                    .unwrap();

                let material_instance = mesh_material.material_instance();
                let pass_indices = MeshBasicShaderPassIndices::new(&material_instance.material);

                Some(MeshBasicAssetPart {
                    mesh_material: mesh_material.clone(),
                    pass_indices,
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

        let inner = MeshBasicAssetInner {
            vertex_full_buffer,
            vertex_position_buffer,
            index_buffer,
            asset_data: mesh_asset,
            mesh_parts,
        };

        Ok(MeshBasicAsset {
            inner: Arc::new(inner),
        })
    }
}

pub type MeshBasicAssetType =
    DefaultAssetTypeHandler<MeshBasicAssetData, MeshBasicAsset, MeshBasicLoadHandler>;
