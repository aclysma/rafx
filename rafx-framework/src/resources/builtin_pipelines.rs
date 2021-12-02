use crate::{
    BufferResource, ComputePipelineResource, CookedShaderPackage, DescriptorSetAllocatorRef,
    DescriptorSetBindings, ResourceArc, ResourceContext, ResourceLookupSet,
};
use rafx_api::{RafxCommandBuffer, RafxResult};
use std::ops::Deref;
use std::sync::Arc;

pub struct BuiltinPipelinesInner {
    pub util_fill_buffer_pipeline: ResourceArc<ComputePipelineResource>,
}

#[derive(Clone)]
pub struct BuiltinPipelines {
    pub inner: Arc<BuiltinPipelinesInner>,
}

impl Deref for BuiltinPipelines {
    type Target = BuiltinPipelinesInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl BuiltinPipelines {
    pub fn new(resources: &ResourceLookupSet) -> RafxResult<Self> {
        let util_fill_buffer =
            bincode::deserialize::<CookedShaderPackage>(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/cooked_shaders/util_fill_buffer.comp.cookedshaderpackage"
            )))
            .map_err(|x| format!("Failed to deserialize cooked shader: {:?}", x))?;

        let util_fill_buffer_pipeline =
            util_fill_buffer.load_compute_pipeline(resources, "main")?;

        let inner = BuiltinPipelinesInner {
            util_fill_buffer_pipeline,
        };

        Ok(BuiltinPipelines {
            inner: Arc::new(inner),
        })
    }

    pub fn fill_buffer_compute_pass(
        &self,
        command_buffer: &RafxCommandBuffer,
        resource_context: &ResourceContext,
        buffer: &ResourceArc<BufferResource>,
        fill_value: u32,
    ) -> RafxResult<()> {
        let mut descriptor_set_allocator = resource_context.create_descriptor_set_allocator();

        command_buffer.cmd_bind_pipeline(&*self.util_fill_buffer_pipeline.get_raw().pipeline)?;
        self.do_fill_buffer_compute_pass(
            command_buffer,
            &mut descriptor_set_allocator,
            buffer,
            fill_value,
        )
    }

    pub(crate) fn do_fill_buffer_compute_pass(
        &self,
        command_buffer: &RafxCommandBuffer,
        descriptor_set_allocator: &mut DescriptorSetAllocatorRef,
        buffer: &ResourceArc<BufferResource>,
        fill_value: u32,
    ) -> RafxResult<()> {
        let mut descriptor_set = descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(
            &self
                .util_fill_buffer_pipeline
                .get_raw()
                .descriptor_set_layouts[0],
        )?;
        let buffer_bytes_div_by_four: u32 = (buffer.get_raw().buffer.buffer_def().size / 4) as u32;

        use crate::shaders::util_fill_buffer_comp as fill_shader;
        descriptor_set.set_buffer_data(
            fill_shader::CONFIG_DESCRIPTOR_BINDING_INDEX as _,
            &fill_shader::ClearBufferConfigUniform {
                buffer_bytes_div_by_four,
                fill_value,
                ..Default::default()
            },
        );
        descriptor_set.set_buffer(fill_shader::DATA_DESCRIPTOR_BINDING_INDEX as _, buffer);
        descriptor_set.flush(descriptor_set_allocator)?;
        descriptor_set_allocator.flush_changes()?;
        descriptor_set.bind(&command_buffer)?;
        command_buffer.cmd_dispatch(buffer_bytes_div_by_four, 1, 1)
    }
}
