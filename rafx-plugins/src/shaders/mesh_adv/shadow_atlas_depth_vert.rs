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
pub struct PerViewDataStd140 {
    pub view: [[f32; 4]; 4],      // +0 (size: 64)
    pub view_proj: [[f32; 4]; 4], // +64 (size: 64)
    pub uv_min: [f32; 2],         // +128 (size: 8)
    pub uv_max: [f32; 2],         // +136 (size: 8)
} // 144 bytes

impl Default for PerViewDataStd140 {
    fn default() -> Self {
        PerViewDataStd140 {
            view: <[[f32; 4]; 4]>::default(),
            view_proj: <[[f32; 4]; 4]>::default(),
            uv_min: <[f32; 2]>::default(),
            uv_max: <[f32; 2]>::default(),
        }
    }
}

pub type PerViewDataUniform = PerViewDataStd140;

pub const PER_VIEW_DATA_DESCRIPTOR_SET_INDEX: usize = 0;
pub const PER_VIEW_DATA_DESCRIPTOR_BINDING_INDEX: usize = 0;
pub const ALL_TRANSFORMS_DESCRIPTOR_SET_INDEX: usize = 1;
pub const ALL_TRANSFORMS_DESCRIPTOR_BINDING_INDEX: usize = 0;
pub const ALL_DRAW_DATA_DESCRIPTOR_SET_INDEX: usize = 1;
pub const ALL_DRAW_DATA_DESCRIPTOR_BINDING_INDEX: usize = 1;

pub struct DescriptorSet0Args<'a> {
    pub per_view_data: &'a PerViewDataUniform,
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
    }

    pub fn set_args(
        &mut self,
        args: DescriptorSet0Args,
    ) {
        self.set_per_view_data(args.per_view_data);
    }

    pub fn set_per_view_data(
        &mut self,
        per_view_data: &PerViewDataUniform,
    ) {
        self.0
            .set_buffer_data(PER_VIEW_DATA_DESCRIPTOR_BINDING_INDEX as u32, per_view_data);
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
    fn test_struct_per_view_data_std140() {
        assert_eq!(std::mem::size_of::<PerViewDataStd140>(), 144);
        assert_eq!(std::mem::size_of::<[[f32; 4]; 4]>(), 64);
        assert_eq!(std::mem::align_of::<[[f32; 4]; 4]>(), 4);
        assert_eq!(memoffset::offset_of!(PerViewDataStd140, view), 0);
        assert_eq!(std::mem::size_of::<[[f32; 4]; 4]>(), 64);
        assert_eq!(std::mem::align_of::<[[f32; 4]; 4]>(), 4);
        assert_eq!(memoffset::offset_of!(PerViewDataStd140, view_proj), 64);
        assert_eq!(std::mem::size_of::<[f32; 2]>(), 8);
        assert_eq!(std::mem::align_of::<[f32; 2]>(), 4);
        assert_eq!(memoffset::offset_of!(PerViewDataStd140, uv_min), 128);
        assert_eq!(std::mem::size_of::<[f32; 2]>(), 8);
        assert_eq!(std::mem::align_of::<[f32; 2]>(), 4);
        assert_eq!(memoffset::offset_of!(PerViewDataStd140, uv_max), 136);
    }
}
