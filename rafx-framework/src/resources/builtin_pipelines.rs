use crate::shaders::util_blit_image;
use crate::VertexDataSetLayout;
use crate::{
    BufferResource, ComputePipelineResource, DescriptorSetAllocatorRef, DescriptorSetBindings,
    FixedFunctionState, GraphicsPipelineRenderTargetMeta, ImageViewResource, MaterialPassResource,
    ResourceArc, ResourceContext, ResourceLookupSet,
};
use rafx_api::{
    RafxBlendStateRenderTarget, RafxCommandBuffer, RafxHashedShaderPackage, RafxPrimitiveTopology,
    RafxResult,
};
use std::ops::Deref;
use std::sync::Arc;

lazy_static::lazy_static! {
    pub static ref EMPTY_VERTEX_LAYOUT : VertexDataSetLayout = {
        VertexDataSetLayout::new(vec![], RafxPrimitiveTopology::TriangleList)
    };
}

pub struct BuiltinPipelinesInner {
    pub util_fill_buffer_pipeline: ResourceArc<ComputePipelineResource>,
    pub util_blit_image_material_pass: ResourceArc<MaterialPassResource>,
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
        let util_fill_buffer_pipeline = Self::create_util_fill_buffer_pass(resources)?;
        let util_blit_image_material_pass = Self::create_util_blit_image_pass(resources)?;

        let inner = BuiltinPipelinesInner {
            util_fill_buffer_pipeline,
            util_blit_image_material_pass,
        };

        Ok(BuiltinPipelines {
            inner: Arc::new(inner),
        })
    }

    fn create_util_fill_buffer_pass(
        resources: &ResourceLookupSet
    ) -> RafxResult<ResourceArc<ComputePipelineResource>> {
        let util_fill_buffer =
            bincode::deserialize::<RafxHashedShaderPackage>(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/cooked_shaders/util_fill_buffer.comp.cookedshaderpackage"
            )))
            .map_err(|x| format!("Failed to deserialize cooked shader: {:?}", x))?;

        super::cooked_shader::load_compute_pipeline(&util_fill_buffer, resources, "main")
    }

    fn create_util_blit_image_pass(
        resources: &ResourceLookupSet
    ) -> RafxResult<ResourceArc<MaterialPassResource>> {
        let util_blit_image_vert =
            bincode::deserialize::<RafxHashedShaderPackage>(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/cooked_shaders/util_blit_image/util_blit_image.vert.cookedshaderpackage"
            )))
            .map_err(|x| format!("Failed to deserialize cooked shader: {:?}", x))?;

        let util_blit_image_frag =
            bincode::deserialize::<RafxHashedShaderPackage>(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/cooked_shaders/util_blit_image/util_blit_image.frag.cookedshaderpackage"
            )))
            .map_err(|x| format!("Failed to deserialize cooked shader: {:?}", x))?;

        let mut util_blit_image_fixed_function_state = FixedFunctionState {
            blend_state: Default::default(),
            depth_state: Default::default(),
            rasterizer_state: Default::default(),
        };
        util_blit_image_fixed_function_state
            .blend_state
            .independent_blend = false;
        util_blit_image_fixed_function_state
            .blend_state
            .render_target_blend_states = vec![RafxBlendStateRenderTarget::default_alpha_enabled()];

        super::cooked_shader::load_material_pass(
            resources,
            &[&util_blit_image_vert, &util_blit_image_frag],
            &["main", "main"],
            Arc::new(util_blit_image_fixed_function_state),
        )
    }

    pub fn fill_buffer(
        &self,
        command_buffer: &RafxCommandBuffer,
        resource_context: &ResourceContext,
        buffer: &ResourceArc<BufferResource>,
        fill_value: u32,
    ) -> RafxResult<()> {
        let buffer_bytes_div_by_four: u32 = (buffer.get_raw().buffer.buffer_def().size / 4) as u32;

        // Keep in sync with group size in the shader
        const GROUP_SIZE_X: u32 = 64;

        // We need at least this many groups
        let num_workgroups = buffer_bytes_div_by_four / GROUP_SIZE_X;

        let (group_count_x, group_count_y) = if num_workgroups <= 1 {
            // For group sizes that are small enough,
            (num_workgroups, 1)
        } else {
            // We dispatch in XY shape because some GPUs have a limit of 65k in a single dimension.
            // This method will ensure we don't dispatch any more than group_count_isqrt-1 unnecessary
            // workgroups

            // Estimate largest integer <= sqrt(num_workgroups) with fudge for FP accuracy
            // This may panic due to wrapping if num_workgroups is > ~4B
            let mut group_count_isqrt = ((num_workgroups as f32).sqrt() - 0.0001).floor() as u32;
            while group_count_isqrt * group_count_isqrt < num_workgroups {
                group_count_isqrt += 1;
            }

            if group_count_isqrt * (group_count_isqrt - 1) >= num_workgroups {
                (group_count_isqrt, group_count_isqrt - 1)
            } else {
                (group_count_isqrt, group_count_isqrt)
            }
        };

        let mut descriptor_set_allocator = resource_context.create_descriptor_set_allocator();

        command_buffer.cmd_bind_pipeline(&*self.util_fill_buffer_pipeline.get_raw().pipeline)?;

        let mut descriptor_set = descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(
            &self
                .util_fill_buffer_pipeline
                .get_raw()
                .descriptor_set_layouts[0],
        )?;

        use crate::shaders::util_fill_buffer_comp as fill_shader;
        descriptor_set.set_buffer_data(
            fill_shader::CONFIG_DESCRIPTOR_BINDING_INDEX as _,
            &fill_shader::ClearBufferConfigUniform {
                buffer_bytes_div_by_four,
                fill_value,
                num_workgroups_x: group_count_x,
                ..Default::default()
            },
        );
        descriptor_set.set_buffer(fill_shader::DATA_DESCRIPTOR_BINDING_INDEX as _, buffer);
        descriptor_set.flush(&mut descriptor_set_allocator)?;
        descriptor_set_allocator.flush_changes()?;
        descriptor_set.bind(&command_buffer)?;

        command_buffer.cmd_dispatch(group_count_x, group_count_y, 1)
    }

    pub fn blit_image(
        &self,
        command_buffer: &RafxCommandBuffer,
        resource_context: &ResourceContext,
        render_target_meta: &GraphicsPipelineRenderTargetMeta,
        src_image: &ResourceArc<ImageViewResource>,
        src_uv_min: glam::Vec2,
        src_uv_max: glam::Vec2,
        dst_image: &ResourceArc<ImageViewResource>,
        dst_uv_min: glam::Vec2,
        dst_uv_max: glam::Vec2,
    ) -> RafxResult<()> {
        let mut descriptor_set_allocator = resource_context.create_descriptor_set_allocator();

        let dst_image_extents = dst_image
            .get_raw()
            .image
            .get_raw()
            .image
            .texture_def()
            .extents;
        let dst_offset_pixels = glam::Vec2::new(
            dst_uv_min.x * dst_image_extents.width as f32,
            dst_uv_min.y * dst_image_extents.height as f32,
        );
        let dst_size_pixels = glam::Vec2::new(
            (dst_uv_max.x - dst_uv_min.x) * dst_image_extents.width as f32,
            (dst_uv_max.y - dst_uv_min.y) * dst_image_extents.height as f32,
        );

        self.do_blit_image(
            command_buffer,
            resource_context,
            render_target_meta,
            &mut descriptor_set_allocator,
            src_image,
            src_uv_min,
            src_uv_max,
            dst_offset_pixels,
            dst_size_pixels,
        )
    }

    pub(crate) fn do_blit_image(
        &self,
        command_buffer: &RafxCommandBuffer,
        resource_context: &ResourceContext,
        render_target_meta: &GraphicsPipelineRenderTargetMeta,
        descriptor_set_allocator: &mut DescriptorSetAllocatorRef,
        src_image: &ResourceArc<ImageViewResource>,
        src_uv_min: glam::Vec2,
        src_uv_max: glam::Vec2,
        dst_offset_pixels: glam::Vec2,
        dst_size_pixels: glam::Vec2,
    ) -> RafxResult<()> {
        let pipeline = resource_context
            .graphics_pipeline_cache()
            .get_or_create_graphics_pipeline(
                None,
                &self.util_blit_image_material_pass,
                render_target_meta,
                &EMPTY_VERTEX_LAYOUT,
            )?;

        command_buffer.cmd_bind_pipeline(&pipeline.get_raw().pipeline)?;

        let descriptor_set_layouts = self
            .util_blit_image_material_pass
            .get_raw()
            .descriptor_set_layouts;

        use util_blit_image::util_blit_image_frag;
        let descriptor_set = descriptor_set_allocator.create_descriptor_set_with_writer(
            &descriptor_set_layouts[util_blit_image_frag::SRC_TEX_DESCRIPTOR_SET_INDEX],
            util_blit_image_frag::DescriptorSet0Args {
                src_tex: &src_image,
                config: &util_blit_image_frag::ConfigUniform {
                    src_uv_min: src_uv_min.into(),
                    src_uv_max: src_uv_max.into(),
                },
            },
        )?;

        descriptor_set_allocator.flush_changes()?;

        descriptor_set.bind(command_buffer)?;
        command_buffer.cmd_set_viewport(
            dst_offset_pixels.x,
            dst_offset_pixels.y,
            dst_size_pixels.x,
            dst_size_pixels.y,
            0.0,
            1.0,
        )?;
        command_buffer.cmd_draw(3, 0)?;
        Ok(())
    }
}
