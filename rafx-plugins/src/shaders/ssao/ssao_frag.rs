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
pub struct ConfigStd140 {
    pub proj: [[f32; 4]; 4],           // +0 (size: 64)
    pub proj_inv: [[f32; 4]; 4],       // +64 (size: 64)
    pub samples: [[f32; 4]; 16],       // +128 (size: 256)
    pub random_noise_offset: [f32; 2], // +384 (size: 8)
    pub frame_index: u32,              // +392 (size: 4)
    pub _padding0: [u8; 4],            // +396 (size: 4)
} // 400 bytes

impl Default for ConfigStd140 {
    fn default() -> Self {
        ConfigStd140 {
            proj: <[[f32; 4]; 4]>::default(),
            proj_inv: <[[f32; 4]; 4]>::default(),
            samples: [<[f32; 4]>::default(); 16],
            random_noise_offset: <[f32; 2]>::default(),
            frame_index: <u32>::default(),
            _padding0: [u8::default(); 4],
        }
    }
}

pub type ConfigUniform = ConfigStd140;

pub const DEPTH_TEX_DESCRIPTOR_SET_INDEX: usize = 0;
pub const DEPTH_TEX_DESCRIPTOR_BINDING_INDEX: usize = 0;
pub const NOISE_TEX_DESCRIPTOR_SET_INDEX: usize = 0;
pub const NOISE_TEX_DESCRIPTOR_BINDING_INDEX: usize = 1;
pub const CONFIG_DESCRIPTOR_SET_INDEX: usize = 0;
pub const CONFIG_DESCRIPTOR_BINDING_INDEX: usize = 2;
pub const SMP_NEAREST_DESCRIPTOR_SET_INDEX: usize = 0;
pub const SMP_NEAREST_DESCRIPTOR_BINDING_INDEX: usize = 3;
pub const SMP_LINEAR_DESCRIPTOR_SET_INDEX: usize = 0;
pub const SMP_LINEAR_DESCRIPTOR_BINDING_INDEX: usize = 4;

pub struct DescriptorSet0Args<'a> {
    pub depth_tex: &'a ResourceArc<ImageViewResource>,
    pub noise_tex: &'a ResourceArc<ImageViewResource>,
    pub config: &'a ConfigUniform,
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
        descriptor_set.set_image(DEPTH_TEX_DESCRIPTOR_BINDING_INDEX as u32, args.depth_tex);
        descriptor_set.set_image(NOISE_TEX_DESCRIPTOR_BINDING_INDEX as u32, args.noise_tex);
        descriptor_set.set_buffer_data(CONFIG_DESCRIPTOR_BINDING_INDEX as u32, args.config);
    }
}

pub struct DescriptorSet0(pub DynDescriptorSet);

impl DescriptorSet0 {
    pub fn set_args_static(
        descriptor_set: &mut DynDescriptorSet,
        args: DescriptorSet0Args,
    ) {
        descriptor_set.set_image(DEPTH_TEX_DESCRIPTOR_BINDING_INDEX as u32, args.depth_tex);
        descriptor_set.set_image(NOISE_TEX_DESCRIPTOR_BINDING_INDEX as u32, args.noise_tex);
        descriptor_set.set_buffer_data(CONFIG_DESCRIPTOR_BINDING_INDEX as u32, args.config);
    }

    pub fn set_args(
        &mut self,
        args: DescriptorSet0Args,
    ) {
        self.set_depth_tex(args.depth_tex);
        self.set_noise_tex(args.noise_tex);
        self.set_config(args.config);
    }

    pub fn set_depth_tex(
        &mut self,
        depth_tex: &ResourceArc<ImageViewResource>,
    ) {
        self.0
            .set_image(DEPTH_TEX_DESCRIPTOR_BINDING_INDEX as u32, depth_tex);
    }

    pub fn set_noise_tex(
        &mut self,
        noise_tex: &ResourceArc<ImageViewResource>,
    ) {
        self.0
            .set_image(NOISE_TEX_DESCRIPTOR_BINDING_INDEX as u32, noise_tex);
    }

    pub fn set_config(
        &mut self,
        config: &ConfigUniform,
    ) {
        self.0
            .set_buffer_data(CONFIG_DESCRIPTOR_BINDING_INDEX as u32, config);
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
    fn test_struct_config_std140() {
        assert_eq!(std::mem::size_of::<ConfigStd140>(), 400);
        assert_eq!(std::mem::size_of::<[[f32; 4]; 4]>(), 64);
        assert_eq!(std::mem::align_of::<[[f32; 4]; 4]>(), 4);
        assert_eq!(memoffset::offset_of!(ConfigStd140, proj), 0);
        assert_eq!(std::mem::size_of::<[[f32; 4]; 4]>(), 64);
        assert_eq!(std::mem::align_of::<[[f32; 4]; 4]>(), 4);
        assert_eq!(memoffset::offset_of!(ConfigStd140, proj_inv), 64);
        assert_eq!(std::mem::size_of::<[[f32; 4]; 16]>(), 256);
        assert_eq!(std::mem::align_of::<[[f32; 4]; 16]>(), 4);
        assert_eq!(memoffset::offset_of!(ConfigStd140, samples), 128);
        assert_eq!(std::mem::size_of::<[f32; 2]>(), 8);
        assert_eq!(std::mem::align_of::<[f32; 2]>(), 4);
        assert_eq!(
            memoffset::offset_of!(ConfigStd140, random_noise_offset),
            384
        );
        assert_eq!(std::mem::size_of::<u32>(), 4);
        assert_eq!(std::mem::align_of::<u32>(), 4);
        assert_eq!(memoffset::offset_of!(ConfigStd140, frame_index), 392);
        assert_eq!(std::mem::size_of::<[u8; 4]>(), 4);
        assert_eq!(std::mem::align_of::<[u8; 4]>(), 1);
        assert_eq!(memoffset::offset_of!(ConfigStd140, _padding0), 396);
    }
}
