use crate::demo_phases::*;
use glam::Vec3;
use legion::*;
use rafx::nodes::RenderRegistryBuilder;
use rafx::nodes::RenderViewSet;
use rafx::nodes::{AllRenderNodes, ExtractJobSet, FramePacketBuilder, RenderPhaseMaskBuilder};
use rafx::visibility::*;

#[derive(Copy, Clone)]
pub struct PositionComponent {
    pub position: Vec3,
}

mod demo_feature;
use demo_feature::*;
use rafx_nodes::RenderViewDepthRange;

mod demo_phases;

//
// Just for demonstration of minimal API
//
pub struct DemoExtractContext {
    world: &'static World,
    resources: &'static Resources,
}

impl DemoExtractContext {
    pub fn new<'a>(
        world: &'a World,
        resources: &'a Resources,
    ) -> Self {
        unsafe {
            DemoExtractContext {
                world: force_to_static_lifetime(world),
                resources: force_to_static_lifetime(resources),
            }
        }
    }
}

unsafe fn force_to_static_lifetime<T>(value: &T) -> &'static T {
    std::mem::transmute(value)
}

pub struct DemoPrepareContext;
pub struct DemoWriteContext;

#[derive(Clone)]
pub struct DemoComponent {
    pub render_node: DemoRenderNodeHandle,
    pub visibility_node: DynamicAabbVisibilityNodeHandle,
    pub alpha: f32,
}

fn main() {
    // Setup logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    //
    // Setup render features
    //
    let render_registry = RenderRegistryBuilder::default()
        .register_feature::<DemoRenderFeature>()
        .register_render_phase::<DemoOpaqueRenderPhase>("Opaque")
        .register_render_phase::<DemoTransparentRenderPhase>("Transparent")
        .build();

    //
    // Set up render phase masks for each view. This is used to enable/disable phases for particular
    // view. For example this would be used to pick a different pipeline for rendering shadow maps
    //
    let main_camera_render_phase_mask = RenderPhaseMaskBuilder::default()
        .add_render_phase::<DemoOpaqueRenderPhase>()
        .add_render_phase::<DemoTransparentRenderPhase>()
        .build();

    let minimap_render_phase_mask = RenderPhaseMaskBuilder::default()
        .add_render_phase::<DemoOpaqueRenderPhase>()
        .add_render_phase::<DemoTransparentRenderPhase>()
        .build();

    // In theory we could pre-cook static visibility in chunks and stream them in
    let mut static_visibility_node_set = StaticVisibilityNodeSet::default();
    let mut dynamic_visibility_node_set = DynamicVisibilityNodeSet::default();
    let demo_render_nodes = DemoRenderNodeSet::default();

    //
    // Init an example world state
    //
    let mut world = World::default();
    let mut resources = Resources::default();

    resources.insert(demo_render_nodes);

    {
        // This could be data like references to sprite assets
        let demo_component_names = ["sprite1", "sprite2", "sprite3"];
        for i in 0..100 {
            let position = Vec3::new(((i / 10) * 100) as f32, ((i % 10) * 100) as f32, 0.0);
            let alpha = if i % 7 == 0 { 0.50 } else { 1.0 };
            let _demo_component_names = demo_component_names[i % demo_component_names.len()];

            let mut demo_render_nodes = resources.get_mut::<DemoRenderNodeSet>().unwrap();

            // User calls functions to register render objects
            // - This is a retained API because render object existence can trigger loading streaming assets and
            //   keep them resident in memory
            // - Some render objects might not correspond to legion entities, and some people might not be using
            //   legion at all
            // - the `_with_handle` variant allows us to get the handle of the value that's going to be allocated
            //   This resolves a circular dependency where the component needs the render node handle and the
            //   render node needs the entity.
            // - ALTERNATIVE: Could create an empty entity, create the components, and then add all of them
            let render_node = demo_render_nodes.register_demo_component(DemoRenderNode {
                // Whatever is necessary to render here
                position,
                alpha,
            });

            // User calls functions to register visibility objects
            // - This is a retained API because presumably we don't want to rebuild spatial structures every frame
            let visibility_node =
                dynamic_visibility_node_set.register_dynamic_aabb(DynamicAabbVisibilityNode {
                    handle: render_node.as_raw_generic_handle(),
                    // aabb bounds
                });

            let position_component = PositionComponent { position };
            let demo_component = DemoComponent {
                render_node,
                visibility_node,
                alpha,
            };

            let entity =
                world.extend((0..1).map(|_| (position_component, demo_component.clone())))[0];

            println!("create entity {:?}", entity);
        }
    }

    //
    // Update loop example
    //
    for _ in 0..1 {
        println!("----- FRAME -----");

        // One view set per frame
        let render_view_set = RenderViewSet::default();

        //
        // Take input
        //

        //
        // Calculate user camera
        //

        let eye_position = glam::Vec3::from([0.0, 0.0, 5.0]);
        // User calls function to create "main" view
        let view = glam::Mat4::look_at_rh(
            eye_position,
            glam::Vec3::from([0.0, 0.0, 0.0]),
            glam::Vec3::from([0.0, 1.0, 0.0]),
        );

        let frustum_width = 800;
        let frustum_height = 600;
        let near = 0.1;
        let far = 100.0;
        let projection = glam::Mat4::orthographic_rh(
            0.0,
            frustum_width as f32,
            0.0,
            frustum_height as f32,
            near,
            far,
        );

        let view_proj = projection * view;

        println!("eye is at {}", view_proj);

        let main_view = render_view_set.create_view(
            eye_position,
            view_proj,
            glam::Mat4::identity(),
            (frustum_width, frustum_height),
            RenderViewDepthRange::new(near, far),
            main_camera_render_phase_mask,
            "main".to_string(),
        );

        //
        // Predict visibility for static objects.. this could be front-loaded ahead of simulation to reduce latency
        // Should also consider pre-cached/serialized visibility data that might be streamed in/out in chunks. Updates
        // to static visibility would have to happen before this point. This could be as simple as pushing a
        // pre-built visibility data structure loaded from disk into a list.
        //

        // User could call function to calculate visibility of static objects for FPS camera early to reduce
        // future critical-path work (to reduce latency). The bungie talk took a volume of possible camera
        // positions instead of a single position
        let main_view_static_visibility_result =
            static_visibility_node_set.calculate_static_visibility(&main_view); // return task?

        //
        // Simulation would go here
        //

        //
        // Figure out other views (example: minimap, shadow maps, etc.)
        //
        let minimap_view = render_view_set.create_view(
            eye_position,
            view_proj,
            glam::Mat4::identity(),
            (frustum_width, frustum_height),
            RenderViewDepthRange::new(near, far),
            minimap_render_phase_mask,
            "minimap".to_string(),
        );

        //
        // Finish visibility calculations for all views. Views can be processed in their own jobs.
        //

        // User calls functions to start jobs that calculate dynamic visibility for FPS view
        let main_view_dynamic_visibility_result =
            dynamic_visibility_node_set.calculate_dynamic_visibility(&main_view);

        // User calls functions to start jobs that calculate static and dynamic visibility for all other views
        let minimap_static_visibility_result =
            static_visibility_node_set.calculate_static_visibility(&minimap_view);
        let minimap_dynamic_visibility_result =
            dynamic_visibility_node_set.calculate_dynamic_visibility(&minimap_view);

        log::trace!(
            "main view static node count: {}",
            main_view_static_visibility_result.handles.len()
        );
        log::trace!(
            "main view dynamic node count: {}",
            main_view_dynamic_visibility_result.handles.len()
        );

        //
        // Populate the frame packet. Views can potentially be run in their own jobs in the future.
        // There is a single sync point after this to give features a callback that extraction is about to begin.
        // (All frame nodes must be allocated before this point). After that, extraction for all features and views
        // can run in parallel.
        //
        // The frame packet builder will merge visibility results and hold extracted data from simulation. Once
        // the frame packet is built, render nodes can't be added/removed until extraction is complete
        //

        // After this point, are not allowed to add/remove render nodes until extraction is complete
        let frame_packet_builder = {
            let mut demo_render_nodes = resources.get_mut::<DemoRenderNodeSet>().unwrap();
            demo_render_nodes.update();
            let mut all_render_nodes = AllRenderNodes::default();
            all_render_nodes.add_render_nodes(&*demo_render_nodes);

            FramePacketBuilder::new(&all_render_nodes)
        };

        // After these jobs end, user calls functions to start jobs that extract data
        frame_packet_builder.add_view(
            &main_view,
            &[
                main_view_static_visibility_result,
                main_view_dynamic_visibility_result,
            ],
        );

        frame_packet_builder.add_view(
            &minimap_view,
            &[
                minimap_static_visibility_result,
                minimap_dynamic_visibility_result,
            ],
        );

        //
        // Run extraction jobs for all views/features
        //

        // Up to end user if they want to create every frame or cache somewhere. Letting the user
        // create the feature impls per frame allows them to make system-level data available to
        // the callbacks. (Like maybe a reference to world?)
        // let mut extract_impl_set = RenderFeatureExtractImplSet::new();
        // extract_impl_set.add_impl(Box::new(DemoRenderFeature));
        // extract_impl_set.add_impl(Box::new(StaticQuadRenderFeature));

        let frame_packet = frame_packet_builder.build();
        //println!("frame packet:\n{:#?}", frame_packet);

        let prepare_job_set = {
            let mut extract_job_set = ExtractJobSet::new();
            extract_job_set.add_job(create_demo_extract_job());
            // Other features can be added here

            let mut extract_context = DemoExtractContext::new(&world, &resources);
            extract_job_set.extract(
                &mut extract_context,
                &frame_packet,
                &[&main_view, &minimap_view],
            )
        };

        //
        // At this point, we can start the next simulation loop. The renderer has everything it needs
        // to render the game without referring to game state stored in the frame packet or feature renderers.
        // Visibility and render nodes can be modified up to the point that we start doing visibility
        // checks and building the next frame packet
        //

        // This will produce submit nodes for each feature and merge them, grouped by view/phase
        // The submit nodes will be sorted by the the callback on the phase. This could, for example
        // sort transparent stuff back to front, or sort by meshes that could be rendered by
        // instancing
        let prepare_context = DemoPrepareContext;
        let prepared_render_data = prepare_job_set.prepare(
            &prepare_context,
            &frame_packet,
            &[&main_view, &minimap_view],
            &render_registry,
        );

        // At this point the end-user can kick off the final write job per view/phase pair. The
        // output of this is left up to the end user and would likely be something like a GPU
        // command buffer.
        let mut write_context = DemoWriteContext {};

        println!("write view phase DemoOpaqueRenderPhase for main_view");
        prepared_render_data
            .write_view_phase::<DemoOpaqueRenderPhase>(&main_view, &mut write_context);

        println!("write view phase DemoTransparentRenderPhase for main_view");
        prepared_render_data
            .write_view_phase::<DemoTransparentRenderPhase>(&main_view, &mut write_context);

        println!("write view phase DemoOpaqueRenderPhase for minimap_view");
        prepared_render_data
            .write_view_phase::<DemoOpaqueRenderPhase>(&minimap_view, &mut write_context);

        println!("write view phase DemoTransparentRenderPhase for minimap_view");
        prepared_render_data
            .write_view_phase::<DemoTransparentRenderPhase>(&minimap_view, &mut write_context);
    }

    // Unregistration of render nodes/visibility objects is automatic when they drop out of scope
}
