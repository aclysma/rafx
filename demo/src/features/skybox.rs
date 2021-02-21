// Not a full-on proper feature, but we'll put some skybox-specific stuff here anyways

use rafx::api::{RafxCommandBuffer, RafxPrimitiveTopology, RafxResult};
use rafx::framework::{
    GraphicsPipelineRenderTargetMeta, ImageViewResource, MaterialPassResource, ResourceArc,
    ResourceContext, VertexDataSetLayout,
};
use rafx::nodes::{RenderPhaseIndex, RenderView};

lazy_static::lazy_static! {
    pub static ref EMPTY_VERTEX_LAYOUT : VertexDataSetLayout = {
        VertexDataSetLayout::new(vec![], RafxPrimitiveTopology::TriangleList)
    };
}

pub fn draw_skybox(
    resource_context: &ResourceContext,
    skybox_material: &ResourceArc<MaterialPassResource>,
    skybox_texture: &ResourceArc<ImageViewResource>,
    main_view: &RenderView,
    render_target_meta: &GraphicsPipelineRenderTargetMeta,
    command_buffer: &RafxCommandBuffer,
    render_phase_index: RenderPhaseIndex,
) -> RafxResult<()> {
    // Get the pipeline
    let pipeline = resource_context
        .graphics_pipeline_cache()
        .get_or_create_graphics_pipeline(
            render_phase_index,
            &skybox_material,
            render_target_meta,
            &EMPTY_VERTEX_LAYOUT,
        )?;

    // Set up a descriptor set pointing at the image so we can sample from it
    let mut descriptor_set_allocator = resource_context.create_descriptor_set_allocator();

    let descriptor_set_layouts = &pipeline.get_raw().descriptor_set_layouts;
    let skybox_material_dyn_set0 = descriptor_set_allocator
        .create_descriptor_set(
            &descriptor_set_layouts[shaders::skybox_frag::SKYBOX_TEX_DESCRIPTOR_SET_INDEX],
            shaders::skybox_frag::DescriptorSet0Args {
                skybox_tex: &skybox_texture,
            },
        )
        .unwrap();

    // Skyboxes assume Y up and we're Z up, so "fix" it by adding a rotation about X axis.
    // This effectively applies a rotation to the skybox
    let skybox_rotation = glam::Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2);
    let skybox_material_dyn_set1 = descriptor_set_allocator
        .create_descriptor_set(
            &descriptor_set_layouts[shaders::skybox_frag::UNIFORM_BUFFER_DESCRIPTOR_SET_INDEX],
            shaders::skybox_frag::DescriptorSet1Args {
                uniform_buffer: &shaders::skybox_frag::ArgsUniform {
                    inverse_view: (main_view.view_matrix() * skybox_rotation)
                        .inverse()
                        .to_cols_array_2d(),
                    inverse_projection: main_view.projection_matrix().inverse().to_cols_array_2d(),
                },
            },
        )
        .unwrap();

    descriptor_set_allocator.flush_changes().unwrap();

    // Draw calls
    command_buffer.cmd_bind_pipeline(&*pipeline.get_raw().pipeline)?;
    skybox_material_dyn_set0.bind(command_buffer)?;
    skybox_material_dyn_set1.bind(command_buffer)?;

    command_buffer.cmd_draw(3, 0)
}
