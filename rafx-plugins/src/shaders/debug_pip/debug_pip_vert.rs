// This code is auto-generated by the shader processor.

#[allow(unused_imports)]
use rafx::RafxResult;

#[allow(unused_imports)]
use rafx::framework::{
    DescriptorSetAllocator, DescriptorSetArc, DescriptorSetBindings, DescriptorSetInitializer,
    DescriptorSetWriter, DescriptorSetWriterContext, DynDescriptorSet, ImageViewResource,
    ResourceArc,
};

pub const SMP_DESCRIPTOR_SET_INDEX: usize = 0;
pub const SMP_DESCRIPTOR_BINDING_INDEX: usize = 0;
pub const DEBUG_PIP_TEX_DESCRIPTOR_SET_INDEX: usize = 0;
pub const DEBUG_PIP_TEX_DESCRIPTOR_BINDING_INDEX: usize = 1;

pub struct DescriptorSet0Args<'a> {
    pub debug_pip_tex: &'a ResourceArc<ImageViewResource>,
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
        descriptor_set.set_image(
            DEBUG_PIP_TEX_DESCRIPTOR_BINDING_INDEX as u32,
            args.debug_pip_tex,
        );
    }
}

pub struct DescriptorSet0(pub DynDescriptorSet);

impl DescriptorSet0 {
    pub fn set_args_static(
        descriptor_set: &mut DynDescriptorSet,
        args: DescriptorSet0Args,
    ) {
        descriptor_set.set_image(
            DEBUG_PIP_TEX_DESCRIPTOR_BINDING_INDEX as u32,
            args.debug_pip_tex,
        );
    }

    pub fn set_args(
        &mut self,
        args: DescriptorSet0Args,
    ) {
        self.set_debug_pip_tex(args.debug_pip_tex);
    }

    pub fn set_debug_pip_tex(
        &mut self,
        debug_pip_tex: &ResourceArc<ImageViewResource>,
    ) {
        self.0
            .set_image(DEBUG_PIP_TEX_DESCRIPTOR_BINDING_INDEX as u32, debug_pip_tex);
    }

    pub fn flush(
        &mut self,
        descriptor_set_allocator: &mut DescriptorSetAllocator,
    ) -> RafxResult<()> {
        self.0.flush(descriptor_set_allocator)
    }
}
