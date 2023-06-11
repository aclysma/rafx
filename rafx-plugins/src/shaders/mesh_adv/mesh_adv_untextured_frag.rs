// This code is auto-generated by the shader processor.

#[allow(unused_imports)]
use rafx::RafxResult;

#[allow(unused_imports)]
use rafx::framework::{
    DescriptorSetAllocator, DescriptorSetArc, DescriptorSetBindings, DescriptorSetInitializer,
    DescriptorSetWriter, DescriptorSetWriterContext, DynDescriptorSet, ImageViewResource,
    ResourceArc,
};

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct TransformWithHistoryStd140 {
    pub current_model_matrix: [[f32; 4]; 4],  // +0 (size: 64)
    pub previous_model_matrix: [[f32; 4]; 4], // +64 (size: 64)
} // 128 bytes

impl Default for TransformWithHistoryStd140 {
    fn default() -> Self {
        TransformWithHistoryStd140 {
            current_model_matrix: <[[f32; 4]; 4]>::default(),
            previous_model_matrix: <[[f32; 4]; 4]>::default(),
        }
    }
}

pub type TransformWithHistoryUniform = TransformWithHistoryStd140;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct TransformWithHistoryStd430 {
    pub current_model_matrix: [[f32; 4]; 4],  // +0 (size: 64)
    pub previous_model_matrix: [[f32; 4]; 4], // +64 (size: 64)
} // 128 bytes

pub type TransformWithHistoryPushConstant = TransformWithHistoryStd430;

pub type TransformWithHistoryBuffer = TransformWithHistoryStd430;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct MaterialDbEntryStd140 {
    pub base_color_factor: [f32; 4],               // +0 (size: 16)
    pub emissive_factor: [f32; 3],                 // +16 (size: 12)
    pub metallic_factor: f32,                      // +28 (size: 4)
    pub roughness_factor: f32,                     // +32 (size: 4)
    pub normal_texture_scale: f32,                 // +36 (size: 4)
    pub alpha_threshold: f32,                      // +40 (size: 4)
    pub enable_alpha_blend: u32,                   // +44 (size: 4)
    pub enable_alpha_clip: u32,                    // +48 (size: 4)
    pub color_texture: i32,                        // +52 (size: 4)
    pub base_color_texture_has_alpha_channel: u32, // +56 (size: 4)
    pub metallic_roughness_texture: i32,           // +60 (size: 4)
    pub normal_texture: i32,                       // +64 (size: 4)
    pub emissive_texture: i32,                     // +68 (size: 4)
    pub _padding0: [u8; 8],                        // +72 (size: 8)
} // 80 bytes

impl Default for MaterialDbEntryStd140 {
    fn default() -> Self {
        MaterialDbEntryStd140 {
            base_color_factor: <[f32; 4]>::default(),
            emissive_factor: <[f32; 3]>::default(),
            metallic_factor: <f32>::default(),
            roughness_factor: <f32>::default(),
            normal_texture_scale: <f32>::default(),
            alpha_threshold: <f32>::default(),
            enable_alpha_blend: <u32>::default(),
            enable_alpha_clip: <u32>::default(),
            color_texture: <i32>::default(),
            base_color_texture_has_alpha_channel: <u32>::default(),
            metallic_roughness_texture: <i32>::default(),
            normal_texture: <i32>::default(),
            emissive_texture: <i32>::default(),
            _padding0: [u8::default(); 8],
        }
    }
}

pub type MaterialDbEntryUniform = MaterialDbEntryStd140;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct MaterialDbEntryStd430 {
    pub base_color_factor: [f32; 4],               // +0 (size: 16)
    pub emissive_factor: [f32; 3],                 // +16 (size: 12)
    pub metallic_factor: f32,                      // +28 (size: 4)
    pub roughness_factor: f32,                     // +32 (size: 4)
    pub normal_texture_scale: f32,                 // +36 (size: 4)
    pub alpha_threshold: f32,                      // +40 (size: 4)
    pub enable_alpha_blend: u32,                   // +44 (size: 4)
    pub enable_alpha_clip: u32,                    // +48 (size: 4)
    pub color_texture: i32,                        // +52 (size: 4)
    pub base_color_texture_has_alpha_channel: u32, // +56 (size: 4)
    pub metallic_roughness_texture: i32,           // +60 (size: 4)
    pub normal_texture: i32,                       // +64 (size: 4)
    pub emissive_texture: i32,                     // +68 (size: 4)
    pub _padding0: [u8; 8],                        // +72 (size: 8)
} // 80 bytes

pub type MaterialDbEntryPushConstant = MaterialDbEntryStd430;

pub type MaterialDbEntryBuffer = MaterialDbEntryStd430;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct PerViewDataStd140 {
    pub view: [[f32; 4]; 4],                                 // +0 (size: 64)
    pub view_proj: [[f32; 4]; 4],                            // +64 (size: 64)
    pub ambient_light: [f32; 4],                             // +128 (size: 16)
    pub jitter_amount: [f32; 2],                             // +144 (size: 8)
    pub viewport_width: u32,                                 // +152 (size: 4)
    pub viewport_height: u32,                                // +156 (size: 4)
    pub mip_bias: f32,                                       // +160 (size: 4)
    pub ndf_filter_amount: f32,                              // +164 (size: 4)
    pub directional_light_count: u32,                        // +168 (size: 4)
    pub use_clustered_lighting: u32,                         // +172 (size: 4)
    pub directional_lights: [DirectionalLightStd140; 8],     // +176 (size: 384)
    pub shadow_map_2d_data: [ShadowMap2DDataStd140; 96],     // +560 (size: 9216)
    pub shadow_map_cube_data: [ShadowMapCubeDataStd140; 32], // +9776 (size: 3584)
} // 13360 bytes

impl Default for PerViewDataStd140 {
    fn default() -> Self {
        PerViewDataStd140 {
            view: <[[f32; 4]; 4]>::default(),
            view_proj: <[[f32; 4]; 4]>::default(),
            ambient_light: <[f32; 4]>::default(),
            jitter_amount: <[f32; 2]>::default(),
            viewport_width: <u32>::default(),
            viewport_height: <u32>::default(),
            mip_bias: <f32>::default(),
            ndf_filter_amount: <f32>::default(),
            directional_light_count: <u32>::default(),
            use_clustered_lighting: <u32>::default(),
            directional_lights: [<DirectionalLightStd140>::default(); 8],
            shadow_map_2d_data: [<ShadowMap2DDataStd140>::default(); 96],
            shadow_map_cube_data: [<ShadowMapCubeDataStd140>::default(); 32],
        }
    }
}

pub type PerViewDataUniform = PerViewDataStd140;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct LightBinOutputStd430 {
    pub data: LightBinningOutputStd430, // +0 (size: 6316048)
} // 6316048 bytes

pub type LightBinOutputBuffer = LightBinOutputStd430;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct ShadowMapCubeDataStd140 {
    pub uv_min_uv_max: [[f32; 4]; 6],    // +0 (size: 96)
    pub cube_map_projection_near_z: f32, // +96 (size: 4)
    pub cube_map_projection_far_z: f32,  // +100 (size: 4)
    pub _padding0: [u8; 8],              // +104 (size: 8)
} // 112 bytes

impl Default for ShadowMapCubeDataStd140 {
    fn default() -> Self {
        ShadowMapCubeDataStd140 {
            uv_min_uv_max: [<[f32; 4]>::default(); 6],
            cube_map_projection_near_z: <f32>::default(),
            cube_map_projection_far_z: <f32>::default(),
            _padding0: [u8::default(); 8],
        }
    }
}

pub type ShadowMapCubeDataUniform = ShadowMapCubeDataStd140;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct TransformStd140 {
    pub model_matrix: [[f32; 4]; 4], // +0 (size: 64)
} // 64 bytes

impl Default for TransformStd140 {
    fn default() -> Self {
        TransformStd140 {
            model_matrix: <[[f32; 4]; 4]>::default(),
        }
    }
}

pub type TransformUniform = TransformStd140;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct TransformStd430 {
    pub model_matrix: [[f32; 4]; 4], // +0 (size: 64)
} // 64 bytes

pub type TransformPushConstant = TransformStd430;

pub type TransformBuffer = TransformStd430;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct DrawDataStd140 {
    pub transform_index: u32, // +0 (size: 4)
    pub material_index: u32,  // +4 (size: 4)
    pub _padding0: [u8; 8],   // +8 (size: 8)
} // 16 bytes

impl Default for DrawDataStd140 {
    fn default() -> Self {
        DrawDataStd140 {
            transform_index: <u32>::default(),
            material_index: <u32>::default(),
            _padding0: [u8::default(); 8],
        }
    }
}

pub type DrawDataUniform = DrawDataStd140;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct DrawDataStd430 {
    pub transform_index: u32, // +0 (size: 4)
    pub material_index: u32,  // +4 (size: 4)
} // 8 bytes

pub type DrawDataPushConstant = DrawDataStd430;

pub type DrawDataBuffer = DrawDataStd430;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct ShadowMap2DDataStd140 {
    pub uv_min: [f32; 2],                    // +0 (size: 8)
    pub uv_max: [f32; 2],                    // +8 (size: 8)
    pub shadow_map_view_proj: [[f32; 4]; 4], // +16 (size: 64)
    pub shadow_map_light_dir: [f32; 3],      // +80 (size: 12)
    pub _padding0: [u8; 4],                  // +92 (size: 4)
} // 96 bytes

impl Default for ShadowMap2DDataStd140 {
    fn default() -> Self {
        ShadowMap2DDataStd140 {
            uv_min: <[f32; 2]>::default(),
            uv_max: <[f32; 2]>::default(),
            shadow_map_view_proj: <[[f32; 4]; 4]>::default(),
            shadow_map_light_dir: <[f32; 3]>::default(),
            _padding0: [u8::default(); 4],
        }
    }
}

pub type ShadowMap2DDataUniform = ShadowMap2DDataStd140;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct LightInListStd430 {
    pub position_ws: [f32; 3],            // +0 (size: 12)
    pub range: f32,                       // +12 (size: 4)
    pub position_vs: [f32; 3],            // +16 (size: 12)
    pub intensity: f32,                   // +28 (size: 4)
    pub color: [f32; 4],                  // +32 (size: 16)
    pub spotlight_direction_ws: [f32; 3], // +48 (size: 12)
    pub spotlight_half_angle: f32,        // +60 (size: 4)
    pub spotlight_direction_vs: [f32; 3], // +64 (size: 12)
    pub shadow_map: i32,                  // +76 (size: 4)
} // 80 bytes

pub type LightInListBuffer = LightInListStd430;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct DirectionalLightStd140 {
    pub direction_ws: [f32; 3], // +0 (size: 12)
    pub intensity: f32,         // +12 (size: 4)
    pub color: [f32; 4],        // +16 (size: 16)
    pub direction_vs: [f32; 3], // +32 (size: 12)
    pub shadow_map: i32,        // +44 (size: 4)
} // 48 bytes

impl Default for DirectionalLightStd140 {
    fn default() -> Self {
        DirectionalLightStd140 {
            direction_ws: <[f32; 3]>::default(),
            intensity: <f32>::default(),
            color: <[f32; 4]>::default(),
            direction_vs: <[f32; 3]>::default(),
            shadow_map: <i32>::default(),
        }
    }
}

pub type DirectionalLightUniform = DirectionalLightStd140;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct AllLightsStd430 {
    pub light_count: u32,               // +0 (size: 4)
    pub _padding0: [u8; 12],            // +4 (size: 12)
    pub data: [LightInListStd430; 512], // +16 (size: 40960)
} // 40976 bytes

pub type AllLightsBuffer = AllLightsStd430;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct LightBinningOutputStd430 {
    pub data_write_ptr: u32,                // +0 (size: 4)
    pub pad0: u32,                          // +4 (size: 4)
    pub pad1: u32,                          // +8 (size: 4)
    pub pad2: u32,                          // +12 (size: 4)
    pub offsets: [ClusterMetaStd430; 3072], // +16 (size: 24576)
    pub data: [u32; 1572864],               // +24592 (size: 6291456)
} // 6316048 bytes

pub type LightBinningOutputBuffer = LightBinningOutputStd430;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct ClusterMetaStd430 {
    pub count: u32,       // +0 (size: 4)
    pub first_light: u32, // +4 (size: 4)
} // 8 bytes

pub type ClusterMetaBuffer = ClusterMetaStd430;

pub const PER_VIEW_DATA_DESCRIPTOR_SET_INDEX: usize = 0;
pub const PER_VIEW_DATA_DESCRIPTOR_BINDING_INDEX: usize = 0;
pub const SMP_DESCRIPTOR_SET_INDEX: usize = 0;
pub const SMP_DESCRIPTOR_BINDING_INDEX: usize = 1;
pub const SMP_DEPTH_LINEAR_DESCRIPTOR_SET_INDEX: usize = 0;
pub const SMP_DEPTH_LINEAR_DESCRIPTOR_BINDING_INDEX: usize = 2;
pub const SMP_DEPTH_NEAREST_DESCRIPTOR_SET_INDEX: usize = 0;
pub const SMP_DEPTH_NEAREST_DESCRIPTOR_BINDING_INDEX: usize = 3;
pub const SHADOW_MAP_ATLAS_DESCRIPTOR_SET_INDEX: usize = 0;
pub const SHADOW_MAP_ATLAS_DESCRIPTOR_BINDING_INDEX: usize = 4;
pub const LIGHT_BIN_OUTPUT_DESCRIPTOR_SET_INDEX: usize = 0;
pub const LIGHT_BIN_OUTPUT_DESCRIPTOR_BINDING_INDEX: usize = 5;
pub const ALL_LIGHTS_DESCRIPTOR_SET_INDEX: usize = 0;
pub const ALL_LIGHTS_DESCRIPTOR_BINDING_INDEX: usize = 6;
pub const SSAO_TEXTURE_DESCRIPTOR_SET_INDEX: usize = 1;
pub const SSAO_TEXTURE_DESCRIPTOR_BINDING_INDEX: usize = 0;
pub const ALL_TRANSFORMS_DESCRIPTOR_SET_INDEX: usize = 2;
pub const ALL_TRANSFORMS_DESCRIPTOR_BINDING_INDEX: usize = 0;
pub const ALL_DRAW_DATA_DESCRIPTOR_SET_INDEX: usize = 2;
pub const ALL_DRAW_DATA_DESCRIPTOR_BINDING_INDEX: usize = 1;
pub const ALL_MATERIALS_DESCRIPTOR_SET_INDEX: usize = 3;
pub const ALL_MATERIALS_DESCRIPTOR_BINDING_INDEX: usize = 0;
pub const ALL_MATERIAL_TEXTURES_DESCRIPTOR_SET_INDEX: usize = 3;
pub const ALL_MATERIAL_TEXTURES_DESCRIPTOR_BINDING_INDEX: usize = 1;

pub struct DescriptorSet0Args<'a> {
    pub per_view_data: &'a PerViewDataUniform,
    pub shadow_map_atlas: &'a ResourceArc<ImageViewResource>,
    pub light_bin_output: &'a LightBinOutputBuffer,
    pub all_lights: &'a AllLightsBuffer,
}

impl<'a> DescriptorSetInitializer<'a> for DescriptorSet0Args<'a> {
    type Output = DescriptorSet0;

    fn create_dyn_descriptor_set(
        descriptor_set: DynDescriptorSet,
        args: Self,
    ) -> Self::Output {
        let mut descriptor = DescriptorSet0(descriptor_set);
        descriptor.set_args(args);
        descriptor
    }

    fn create_descriptor_set(
        descriptor_set_allocator: &mut DescriptorSetAllocator,
        descriptor_set: DynDescriptorSet,
        args: Self,
    ) -> RafxResult<DescriptorSetArc> {
        let mut descriptor = Self::create_dyn_descriptor_set(descriptor_set, args);
        descriptor.0.flush(descriptor_set_allocator)?;
        Ok(descriptor.0.descriptor_set().clone())
    }
}

impl<'a> DescriptorSetWriter<'a> for DescriptorSet0Args<'a> {
    fn write_to(
        descriptor_set: &mut DescriptorSetWriterContext,
        args: Self,
    ) {
        descriptor_set.set_buffer_data(
            PER_VIEW_DATA_DESCRIPTOR_BINDING_INDEX as u32,
            args.per_view_data,
        );
        descriptor_set.set_image(
            SHADOW_MAP_ATLAS_DESCRIPTOR_BINDING_INDEX as u32,
            args.shadow_map_atlas,
        );
        descriptor_set.set_buffer_data(
            LIGHT_BIN_OUTPUT_DESCRIPTOR_BINDING_INDEX as u32,
            args.light_bin_output,
        );
        descriptor_set.set_buffer_data(ALL_LIGHTS_DESCRIPTOR_BINDING_INDEX as u32, args.all_lights);
    }
}

pub struct DescriptorSet0(pub DynDescriptorSet);

impl DescriptorSet0 {
    pub fn set_args_static(
        descriptor_set: &mut DynDescriptorSet,
        args: DescriptorSet0Args,
    ) {
        descriptor_set.set_buffer_data(
            PER_VIEW_DATA_DESCRIPTOR_BINDING_INDEX as u32,
            args.per_view_data,
        );
        descriptor_set.set_image(
            SHADOW_MAP_ATLAS_DESCRIPTOR_BINDING_INDEX as u32,
            args.shadow_map_atlas,
        );
        descriptor_set.set_buffer_data(
            LIGHT_BIN_OUTPUT_DESCRIPTOR_BINDING_INDEX as u32,
            args.light_bin_output,
        );
        descriptor_set.set_buffer_data(ALL_LIGHTS_DESCRIPTOR_BINDING_INDEX as u32, args.all_lights);
    }

    pub fn set_args(
        &mut self,
        args: DescriptorSet0Args,
    ) {
        self.set_per_view_data(args.per_view_data);
        self.set_shadow_map_atlas(args.shadow_map_atlas);
        self.set_light_bin_output(args.light_bin_output);
        self.set_all_lights(args.all_lights);
    }

    pub fn set_per_view_data(
        &mut self,
        per_view_data: &PerViewDataUniform,
    ) {
        self.0
            .set_buffer_data(PER_VIEW_DATA_DESCRIPTOR_BINDING_INDEX as u32, per_view_data);
    }

    pub fn set_shadow_map_atlas(
        &mut self,
        shadow_map_atlas: &ResourceArc<ImageViewResource>,
    ) {
        self.0.set_image(
            SHADOW_MAP_ATLAS_DESCRIPTOR_BINDING_INDEX as u32,
            shadow_map_atlas,
        );
    }

    pub fn set_light_bin_output(
        &mut self,
        light_bin_output: &LightBinOutputBuffer,
    ) {
        self.0.set_buffer_data(
            LIGHT_BIN_OUTPUT_DESCRIPTOR_BINDING_INDEX as u32,
            light_bin_output,
        );
    }

    pub fn set_all_lights(
        &mut self,
        all_lights: &AllLightsBuffer,
    ) {
        self.0
            .set_buffer_data(ALL_LIGHTS_DESCRIPTOR_BINDING_INDEX as u32, all_lights);
    }

    pub fn flush(
        &mut self,
        descriptor_set_allocator: &mut DescriptorSetAllocator,
    ) -> RafxResult<()> {
        self.0.flush(descriptor_set_allocator)
    }
}

pub struct DescriptorSet1Args<'a> {
    pub ssao_texture: &'a ResourceArc<ImageViewResource>,
}

impl<'a> DescriptorSetInitializer<'a> for DescriptorSet1Args<'a> {
    type Output = DescriptorSet1;

    fn create_dyn_descriptor_set(
        descriptor_set: DynDescriptorSet,
        args: Self,
    ) -> Self::Output {
        let mut descriptor = DescriptorSet1(descriptor_set);
        descriptor.set_args(args);
        descriptor
    }

    fn create_descriptor_set(
        descriptor_set_allocator: &mut DescriptorSetAllocator,
        descriptor_set: DynDescriptorSet,
        args: Self,
    ) -> RafxResult<DescriptorSetArc> {
        let mut descriptor = Self::create_dyn_descriptor_set(descriptor_set, args);
        descriptor.0.flush(descriptor_set_allocator)?;
        Ok(descriptor.0.descriptor_set().clone())
    }
}

impl<'a> DescriptorSetWriter<'a> for DescriptorSet1Args<'a> {
    fn write_to(
        descriptor_set: &mut DescriptorSetWriterContext,
        args: Self,
    ) {
        descriptor_set.set_image(
            SSAO_TEXTURE_DESCRIPTOR_BINDING_INDEX as u32,
            args.ssao_texture,
        );
    }
}

pub struct DescriptorSet1(pub DynDescriptorSet);

impl DescriptorSet1 {
    pub fn set_args_static(
        descriptor_set: &mut DynDescriptorSet,
        args: DescriptorSet1Args,
    ) {
        descriptor_set.set_image(
            SSAO_TEXTURE_DESCRIPTOR_BINDING_INDEX as u32,
            args.ssao_texture,
        );
    }

    pub fn set_args(
        &mut self,
        args: DescriptorSet1Args,
    ) {
        self.set_ssao_texture(args.ssao_texture);
    }

    pub fn set_ssao_texture(
        &mut self,
        ssao_texture: &ResourceArc<ImageViewResource>,
    ) {
        self.0
            .set_image(SSAO_TEXTURE_DESCRIPTOR_BINDING_INDEX as u32, ssao_texture);
    }

    pub fn flush(
        &mut self,
        descriptor_set_allocator: &mut DescriptorSetAllocator,
    ) -> RafxResult<()> {
        self.0.flush(descriptor_set_allocator)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_struct_transform_with_history_std140() {
        assert_eq!(std::mem::size_of::<TransformWithHistoryStd140>(), 128);
        assert_eq!(std::mem::size_of::<[[f32; 4]; 4]>(), 64);
        assert_eq!(std::mem::align_of::<[[f32; 4]; 4]>(), 4);
        assert_eq!(
            memoffset::offset_of!(TransformWithHistoryStd140, current_model_matrix),
            0
        );
        assert_eq!(std::mem::size_of::<[[f32; 4]; 4]>(), 64);
        assert_eq!(std::mem::align_of::<[[f32; 4]; 4]>(), 4);
        assert_eq!(
            memoffset::offset_of!(TransformWithHistoryStd140, previous_model_matrix),
            64
        );
    }

    #[test]
    fn test_struct_transform_with_history_std430() {
        assert_eq!(std::mem::size_of::<TransformWithHistoryStd430>(), 128);
        assert_eq!(std::mem::size_of::<[[f32; 4]; 4]>(), 64);
        assert_eq!(std::mem::align_of::<[[f32; 4]; 4]>(), 4);
        assert_eq!(
            memoffset::offset_of!(TransformWithHistoryStd430, current_model_matrix),
            0
        );
        assert_eq!(std::mem::size_of::<[[f32; 4]; 4]>(), 64);
        assert_eq!(std::mem::align_of::<[[f32; 4]; 4]>(), 4);
        assert_eq!(
            memoffset::offset_of!(TransformWithHistoryStd430, previous_model_matrix),
            64
        );
    }

    #[test]
    fn test_struct_material_db_entry_std140() {
        assert_eq!(std::mem::size_of::<MaterialDbEntryStd140>(), 80);
        assert_eq!(std::mem::size_of::<[f32; 4]>(), 16);
        assert_eq!(std::mem::align_of::<[f32; 4]>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd140, base_color_factor),
            0
        );
        assert_eq!(std::mem::size_of::<[f32; 3]>(), 12);
        assert_eq!(std::mem::align_of::<[f32; 3]>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd140, emissive_factor),
            16
        );
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(std::mem::align_of::<f32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd140, metallic_factor),
            28
        );
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(std::mem::align_of::<f32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd140, roughness_factor),
            32
        );
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(std::mem::align_of::<f32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd140, normal_texture_scale),
            36
        );
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(std::mem::align_of::<f32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd140, alpha_threshold),
            40
        );
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd140, enable_alpha_blend),
            44
        );
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd140, enable_alpha_clip),
            48
        );
        assert_eq!(std::mem::size_of::<i32>(), 4);
        assert_eq!(std::mem::align_of::<i32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd140, color_texture),
            52
        );
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd140, base_color_texture_has_alpha_channel),
            56
        );
        assert_eq!(std::mem::size_of::<i32>(), 4);
        assert_eq!(std::mem::align_of::<i32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd140, metallic_roughness_texture),
            60
        );
        assert_eq!(std::mem::size_of::<i32>(), 4);
        assert_eq!(std::mem::align_of::<i32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd140, normal_texture),
            64
        );
        assert_eq!(std::mem::size_of::<i32>(), 4);
        assert_eq!(std::mem::align_of::<i32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd140, emissive_texture),
            68
        );
        assert_eq!(std::mem::size_of::<[u8; 8]>(), 8);
        assert_eq!(std::mem::align_of::<[u8; 8]>(), 1);
        assert_eq!(memoffset::offset_of!(MaterialDbEntryStd140, _padding0), 72);
    }

    #[test]
    fn test_struct_material_db_entry_std430() {
        assert_eq!(std::mem::size_of::<MaterialDbEntryStd430>(), 80);
        assert_eq!(std::mem::size_of::<[f32; 4]>(), 16);
        assert_eq!(std::mem::align_of::<[f32; 4]>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd430, base_color_factor),
            0
        );
        assert_eq!(std::mem::size_of::<[f32; 3]>(), 12);
        assert_eq!(std::mem::align_of::<[f32; 3]>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd430, emissive_factor),
            16
        );
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(std::mem::align_of::<f32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd430, metallic_factor),
            28
        );
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(std::mem::align_of::<f32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd430, roughness_factor),
            32
        );
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(std::mem::align_of::<f32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd430, normal_texture_scale),
            36
        );
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(std::mem::align_of::<f32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd430, alpha_threshold),
            40
        );
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd430, enable_alpha_blend),
            44
        );
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd430, enable_alpha_clip),
            48
        );
        assert_eq!(std::mem::size_of::<i32>(), 4);
        assert_eq!(std::mem::align_of::<i32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd430, color_texture),
            52
        );
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd430, base_color_texture_has_alpha_channel),
            56
        );
        assert_eq!(std::mem::size_of::<i32>(), 4);
        assert_eq!(std::mem::align_of::<i32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd430, metallic_roughness_texture),
            60
        );
        assert_eq!(std::mem::size_of::<i32>(), 4);
        assert_eq!(std::mem::align_of::<i32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd430, normal_texture),
            64
        );
        assert_eq!(std::mem::size_of::<i32>(), 4);
        assert_eq!(std::mem::align_of::<i32>(), 4);
        assert_eq!(
            memoffset::offset_of!(MaterialDbEntryStd430, emissive_texture),
            68
        );
        assert_eq!(std::mem::size_of::<[u8; 8]>(), 8);
        assert_eq!(std::mem::align_of::<[u8; 8]>(), 1);
        assert_eq!(memoffset::offset_of!(MaterialDbEntryStd430, _padding0), 72);
    }

    #[test]
    fn test_struct_per_view_data_std140() {
        assert_eq!(std::mem::size_of::<PerViewDataStd140>(), 13360);
        assert_eq!(std::mem::size_of::<[[f32; 4]; 4]>(), 64);
        assert_eq!(std::mem::align_of::<[[f32; 4]; 4]>(), 4);
        assert_eq!(memoffset::offset_of!(PerViewDataStd140, view), 0);
        assert_eq!(std::mem::size_of::<[[f32; 4]; 4]>(), 64);
        assert_eq!(std::mem::align_of::<[[f32; 4]; 4]>(), 4);
        assert_eq!(memoffset::offset_of!(PerViewDataStd140, view_proj), 64);
        assert_eq!(std::mem::size_of::<[f32; 4]>(), 16);
        assert_eq!(std::mem::align_of::<[f32; 4]>(), 4);
        assert_eq!(memoffset::offset_of!(PerViewDataStd140, ambient_light), 128);
        assert_eq!(std::mem::size_of::<[f32; 2]>(), 8);
        assert_eq!(std::mem::align_of::<[f32; 2]>(), 4);
        assert_eq!(memoffset::offset_of!(PerViewDataStd140, jitter_amount), 144);
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(
            memoffset::offset_of!(PerViewDataStd140, viewport_width),
            152
        );
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(
            memoffset::offset_of!(PerViewDataStd140, viewport_height),
            156
        );
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(std::mem::align_of::<f32>(), 4);
        assert_eq!(memoffset::offset_of!(PerViewDataStd140, mip_bias), 160);
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(std::mem::align_of::<f32>(), 4);
        assert_eq!(
            memoffset::offset_of!(PerViewDataStd140, ndf_filter_amount),
            164
        );
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(
            memoffset::offset_of!(PerViewDataStd140, directional_light_count),
            168
        );
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(
            memoffset::offset_of!(PerViewDataStd140, use_clustered_lighting),
            172
        );
        assert_eq!(std::mem::size_of::<[DirectionalLightStd140; 8]>(), 384);
        assert_eq!(std::mem::align_of::<[DirectionalLightStd140; 8]>(), 4);
        assert_eq!(
            memoffset::offset_of!(PerViewDataStd140, directional_lights),
            176
        );
        assert_eq!(std::mem::size_of::<[ShadowMap2DDataStd140; 96]>(), 9216);
        assert_eq!(std::mem::align_of::<[ShadowMap2DDataStd140; 96]>(), 4);
        assert_eq!(
            memoffset::offset_of!(PerViewDataStd140, shadow_map_2d_data),
            560
        );
        assert_eq!(std::mem::size_of::<[ShadowMapCubeDataStd140; 32]>(), 3584);
        assert_eq!(std::mem::align_of::<[ShadowMapCubeDataStd140; 32]>(), 4);
        assert_eq!(
            memoffset::offset_of!(PerViewDataStd140, shadow_map_cube_data),
            9776
        );
    }

    #[test]
    fn test_struct_light_bin_output_std430() {
        assert_eq!(std::mem::size_of::<LightBinOutputStd430>(), 6316048);
        assert_eq!(std::mem::size_of::<LightBinningOutputStd430>(), 6316048);
        assert_eq!(std::mem::align_of::<LightBinningOutputStd430>(), 4);
    }

    #[test]
    fn test_struct_shadow_map_cube_data_std140() {
        assert_eq!(std::mem::size_of::<ShadowMapCubeDataStd140>(), 112);
        assert_eq!(std::mem::size_of::<[[f32; 4]; 6]>(), 96);
        assert_eq!(std::mem::align_of::<[[f32; 4]; 6]>(), 4);
        assert_eq!(
            memoffset::offset_of!(ShadowMapCubeDataStd140, uv_min_uv_max),
            0
        );
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(std::mem::align_of::<f32>(), 4);
        assert_eq!(
            memoffset::offset_of!(ShadowMapCubeDataStd140, cube_map_projection_near_z),
            96
        );
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(std::mem::align_of::<f32>(), 4);
        assert_eq!(
            memoffset::offset_of!(ShadowMapCubeDataStd140, cube_map_projection_far_z),
            100
        );
        assert_eq!(std::mem::size_of::<[u8; 8]>(), 8);
        assert_eq!(std::mem::align_of::<[u8; 8]>(), 1);
        assert_eq!(
            memoffset::offset_of!(ShadowMapCubeDataStd140, _padding0),
            104
        );
    }

    #[test]
    fn test_struct_transform_std140() {
        assert_eq!(std::mem::size_of::<TransformStd140>(), 64);
        assert_eq!(std::mem::size_of::<[[f32; 4]; 4]>(), 64);
        assert_eq!(std::mem::align_of::<[[f32; 4]; 4]>(), 4);
        assert_eq!(memoffset::offset_of!(TransformStd140, model_matrix), 0);
    }

    #[test]
    fn test_struct_transform_std430() {
        assert_eq!(std::mem::size_of::<TransformStd430>(), 64);
        assert_eq!(std::mem::size_of::<[[f32; 4]; 4]>(), 64);
        assert_eq!(std::mem::align_of::<[[f32; 4]; 4]>(), 4);
        assert_eq!(memoffset::offset_of!(TransformStd430, model_matrix), 0);
    }

    #[test]
    fn test_struct_draw_data_std140() {
        assert_eq!(std::mem::size_of::<DrawDataStd140>(), 16);
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(memoffset::offset_of!(DrawDataStd140, transform_index), 0);
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(memoffset::offset_of!(DrawDataStd140, material_index), 4);
        assert_eq!(std::mem::size_of::<[u8; 8]>(), 8);
        assert_eq!(std::mem::align_of::<[u8; 8]>(), 1);
        assert_eq!(memoffset::offset_of!(DrawDataStd140, _padding0), 8);
    }

    #[test]
    fn test_struct_draw_data_std430() {
        assert_eq!(std::mem::size_of::<DrawDataStd430>(), 8);
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(memoffset::offset_of!(DrawDataStd430, transform_index), 0);
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(memoffset::offset_of!(DrawDataStd430, material_index), 4);
    }

    #[test]
    fn test_struct_shadow_map2_d_data_std140() {
        assert_eq!(std::mem::size_of::<ShadowMap2DDataStd140>(), 96);
        assert_eq!(std::mem::size_of::<[f32; 2]>(), 8);
        assert_eq!(std::mem::align_of::<[f32; 2]>(), 4);
        assert_eq!(memoffset::offset_of!(ShadowMap2DDataStd140, uv_min), 0);
        assert_eq!(std::mem::size_of::<[f32; 2]>(), 8);
        assert_eq!(std::mem::align_of::<[f32; 2]>(), 4);
        assert_eq!(memoffset::offset_of!(ShadowMap2DDataStd140, uv_max), 8);
        assert_eq!(std::mem::size_of::<[[f32; 4]; 4]>(), 64);
        assert_eq!(std::mem::align_of::<[[f32; 4]; 4]>(), 4);
        assert_eq!(
            memoffset::offset_of!(ShadowMap2DDataStd140, shadow_map_view_proj),
            16
        );
        assert_eq!(std::mem::size_of::<[f32; 3]>(), 12);
        assert_eq!(std::mem::align_of::<[f32; 3]>(), 4);
        assert_eq!(
            memoffset::offset_of!(ShadowMap2DDataStd140, shadow_map_light_dir),
            80
        );
        assert_eq!(std::mem::size_of::<[u8; 4]>(), 4);
        assert_eq!(std::mem::align_of::<[u8; 4]>(), 1);
        assert_eq!(memoffset::offset_of!(ShadowMap2DDataStd140, _padding0), 92);
    }

    #[test]
    fn test_struct_light_in_list_std430() {
        assert_eq!(std::mem::size_of::<LightInListStd430>(), 80);
        assert_eq!(std::mem::size_of::<[f32; 3]>(), 12);
        assert_eq!(std::mem::align_of::<[f32; 3]>(), 4);
        assert_eq!(memoffset::offset_of!(LightInListStd430, position_ws), 0);
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(std::mem::align_of::<f32>(), 4);
        assert_eq!(memoffset::offset_of!(LightInListStd430, range), 12);
        assert_eq!(std::mem::size_of::<[f32; 3]>(), 12);
        assert_eq!(std::mem::align_of::<[f32; 3]>(), 4);
        assert_eq!(memoffset::offset_of!(LightInListStd430, position_vs), 16);
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(std::mem::align_of::<f32>(), 4);
        assert_eq!(memoffset::offset_of!(LightInListStd430, intensity), 28);
        assert_eq!(std::mem::size_of::<[f32; 4]>(), 16);
        assert_eq!(std::mem::align_of::<[f32; 4]>(), 4);
        assert_eq!(memoffset::offset_of!(LightInListStd430, color), 32);
        assert_eq!(std::mem::size_of::<[f32; 3]>(), 12);
        assert_eq!(std::mem::align_of::<[f32; 3]>(), 4);
        assert_eq!(
            memoffset::offset_of!(LightInListStd430, spotlight_direction_ws),
            48
        );
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(std::mem::align_of::<f32>(), 4);
        assert_eq!(
            memoffset::offset_of!(LightInListStd430, spotlight_half_angle),
            60
        );
        assert_eq!(std::mem::size_of::<[f32; 3]>(), 12);
        assert_eq!(std::mem::align_of::<[f32; 3]>(), 4);
        assert_eq!(
            memoffset::offset_of!(LightInListStd430, spotlight_direction_vs),
            64
        );
        assert_eq!(std::mem::size_of::<i32>(), 4);
        assert_eq!(std::mem::align_of::<i32>(), 4);
        assert_eq!(memoffset::offset_of!(LightInListStd430, shadow_map), 76);
    }

    #[test]
    fn test_struct_directional_light_std140() {
        assert_eq!(std::mem::size_of::<DirectionalLightStd140>(), 48);
        assert_eq!(std::mem::size_of::<[f32; 3]>(), 12);
        assert_eq!(std::mem::align_of::<[f32; 3]>(), 4);
        assert_eq!(
            memoffset::offset_of!(DirectionalLightStd140, direction_ws),
            0
        );
        assert_eq!(std::mem::size_of::<f32>(), 4);
        assert_eq!(std::mem::align_of::<f32>(), 4);
        assert_eq!(memoffset::offset_of!(DirectionalLightStd140, intensity), 12);
        assert_eq!(std::mem::size_of::<[f32; 4]>(), 16);
        assert_eq!(std::mem::align_of::<[f32; 4]>(), 4);
        assert_eq!(memoffset::offset_of!(DirectionalLightStd140, color), 16);
        assert_eq!(std::mem::size_of::<[f32; 3]>(), 12);
        assert_eq!(std::mem::align_of::<[f32; 3]>(), 4);
        assert_eq!(
            memoffset::offset_of!(DirectionalLightStd140, direction_vs),
            32
        );
        assert_eq!(std::mem::size_of::<i32>(), 4);
        assert_eq!(std::mem::align_of::<i32>(), 4);
        assert_eq!(
            memoffset::offset_of!(DirectionalLightStd140, shadow_map),
            44
        );
    }

    #[test]
    fn test_struct_all_lights_std430() {
        assert_eq!(std::mem::size_of::<AllLightsStd430>(), 40976);
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(memoffset::offset_of!(AllLightsStd430, light_count), 0);
        assert_eq!(std::mem::size_of::<[u8; 12]>(), 12);
        assert_eq!(std::mem::align_of::<[u8; 12]>(), 1);
        assert_eq!(memoffset::offset_of!(AllLightsStd430, _padding0), 4);
        assert_eq!(std::mem::size_of::<[LightInListStd430; 512]>(), 40960);
        assert_eq!(std::mem::align_of::<[LightInListStd430; 512]>(), 4);
        assert_eq!(memoffset::offset_of!(AllLightsStd430, data), 16);
    }

    #[test]
    fn test_struct_light_binning_output_std430() {
        assert_eq!(std::mem::size_of::<LightBinningOutputStd430>(), 6316048);
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(std::mem::size_of::<[ClusterMetaStd430; 3072]>(), 24576);
        assert_eq!(std::mem::align_of::<[ClusterMetaStd430; 3072]>(), 4);
        assert_eq!(std::mem::size_of::<[u32; 1572864]>(), 6291456);
        assert_eq!(std::mem::align_of::<[u32; 1572864]>(), 4);
    }

    #[test]
    fn test_struct_cluster_meta_std430() {
        assert_eq!(std::mem::size_of::<ClusterMetaStd430>(), 8);
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(memoffset::offset_of!(ClusterMetaStd430, count), 0);
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(memoffset::offset_of!(ClusterMetaStd430, first_light), 4);
    }
}
