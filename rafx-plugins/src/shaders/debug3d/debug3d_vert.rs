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
pub struct PerFrameUboStd140 {
    pub view_proj: [[f32; 4]; 4], // +0 (size: 64)
} // 64 bytes

impl Default for PerFrameUboStd140 {
    fn default() -> Self {
        PerFrameUboStd140 {
            view_proj: <[[f32; 4]; 4]>::default(),
        }
    }
}

pub type PerFrameUboUniform = PerFrameUboStd140;

pub const PER_FRAME_DATA_DESCRIPTOR_SET_INDEX: usize = 0;
pub const PER_FRAME_DATA_DESCRIPTOR_BINDING_INDEX: usize = 0;

pub struct DescriptorSet0Args<'a> {
    pub per_frame_data: &'a PerFrameUboUniform,
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
            PER_FRAME_DATA_DESCRIPTOR_BINDING_INDEX as u32,
            args.per_frame_data,
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
            PER_FRAME_DATA_DESCRIPTOR_BINDING_INDEX as u32,
            args.per_frame_data,
        );
    }

    pub fn set_args(
        &mut self,
        args: DescriptorSet0Args,
    ) {
        self.set_per_frame_data(args.per_frame_data);
    }

    pub fn set_per_frame_data(
        &mut self,
        per_frame_data: &PerFrameUboUniform,
    ) {
        self.0.set_buffer_data(
            PER_FRAME_DATA_DESCRIPTOR_BINDING_INDEX as u32,
            per_frame_data,
        );
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
    fn test_struct_per_frame_ubo_std140() {
        assert_eq!(std::mem::size_of::<PerFrameUboStd140>(), 64);
        assert_eq!(std::mem::size_of::<[[f32; 4]; 4]>(), 64);
        assert_eq!(std::mem::align_of::<[[f32; 4]; 4]>(), 4);
        assert_eq!(memoffset::offset_of!(PerFrameUboStd140, view_proj), 0);
    }
}
