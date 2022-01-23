use crate::pipelines::modern::graph_generator::ModernPipelineContext;
use crate::shaders::ssao::ssao_frag;
use rafx::api::{RafxFormat, RafxSampleCount};
use rafx::framework::{ImageViewResource, MaterialPassResource, ResourceArc};
use rafx::graph::{RenderGraphImageConstraint, RenderGraphImageUsageId, RenderGraphQueue};
use rafx::renderer::MainViewRenderResource;
use rand::{thread_rng, Rng, SeedableRng};

fn lerp_f32(
    t: f32,
    p0: f32,
    p1: f32,
) -> f32 {
    p0 + (p1 - p0) * t
}

fn generate_samples() -> [[f32; 4]; 16] {
    //NOTE: Tried using a random kernel every frame, it causes lots of flickering. Getting good
    // enough results with a constant kernel and random rotations of it in the shader
    let mut rng = rand::rngs::StdRng::seed_from_u64(1);
    //let mut rng = thread_rng();

    let mut values = [[0.0; 4]; 16];
    for i in 0..values.len() {
        loop {
            let dir = glam::Vec3::new(
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
                rng.gen_range(0.0..1.0),
            );

            if dir.length() <= 1.0 {
                dir.normalize();
                // This biases checking nearby rather than far away for the first few samples
                let length = lerp_f32(i as f32 / values.len() as f32, 0.1, 1.0);
                //let length = 1.0;
                values[i] = (dir * length).extend(1.0).into();
                break;
            }
        }
    }

    values
}

lazy_static::lazy_static! {
    pub static ref SSAO_SAMPLES : [[f32; 4]; 16] = {
        generate_samples()
    };
}

pub(super) struct SsaoPass {
    pub(super) ssao_rt: RenderGraphImageUsageId,
}

pub(super) fn ssao_pass(
    context: &mut ModernPipelineContext,
    ssao_material_pass: &ResourceArc<MaterialPassResource>,
    depth_rt: RenderGraphImageUsageId,
    noise_texture: &ResourceArc<ImageViewResource>,
) -> SsaoPass {
    let node = context
        .graph
        .add_node("SsaoPass", RenderGraphQueue::DefaultGraphics);

    let depth_rt = context.graph.sample_image(
        node,
        depth_rt,
        RenderGraphImageConstraint {
            samples: Some(RafxSampleCount::SampleCount1),
            ..Default::default()
        },
        Default::default(),
    );

    let ssao_rt = context.graph.create_color_attachment(
        node,
        0,
        None,
        RenderGraphImageConstraint {
            format: Some(RafxFormat::R16G16B16A16_SFLOAT),
            ..Default::default()
        },
        Default::default(),
    );

    let ssao_material_pass = ssao_material_pass.clone();
    let noise_texture = noise_texture.clone();
    context.graph.set_renderpass_callback(node, move |args| {
        let depth_tex = args.graph_context.image_view(depth_rt).unwrap();
        let pipeline = args
            .graph_context
            .resource_context()
            .graphics_pipeline_cache()
            .get_or_create_graphics_pipeline(
                None,
                &ssao_material_pass,
                &args.render_target_meta,
                &super::EMPTY_VERTEX_LAYOUT,
            )?;
        let descriptor_set_layouts = &pipeline.get_raw().descriptor_set_layouts;
        let mut descriptor_set_allocator = args
            .graph_context
            .resource_context()
            .create_descriptor_set_allocator();

        let main_view_resource = args
            .graph_context
            .render_resources()
            .fetch::<MainViewRenderResource>();
        let main_view = main_view_resource.main_view.clone().unwrap();
        let proj = main_view.projection_matrix();
        let proj_inv = main_view.projection_matrix().inverse();
        let mut rng = thread_rng();
        let random_noise_offset = [rng.gen_range(0.0..1.0), rng.gen_range(0.0..1.0)];

        let descriptor_set = descriptor_set_allocator.create_descriptor_set(
            &descriptor_set_layouts[ssao_frag::CONFIG_DESCRIPTOR_SET_INDEX],
            ssao_frag::DescriptorSet0Args {
                depth_tex: &depth_tex,
                noise_tex: &noise_texture,
                config: &ssao_frag::ConfigUniform {
                    proj: proj.to_cols_array_2d(),
                    proj_inv: proj_inv.to_cols_array_2d(),
                    samples: *SSAO_SAMPLES,
                    frame_index: main_view.frame_index() as u32,
                    random_noise_offset,
                    _padding0: Default::default(),
                },
            },
        )?;

        // Explicit flush since we're going to use the descriptors immediately
        descriptor_set_allocator.flush_changes()?;

        // Draw calls
        let command_buffer = &args.command_buffer;
        command_buffer.cmd_bind_pipeline(&*pipeline.get_raw().pipeline)?;
        descriptor_set.bind(command_buffer)?;
        command_buffer.cmd_draw(3, 0)?;

        Ok(())
    });

    SsaoPass { ssao_rt }
}
