/*
struct FrameGraphBuilder {

}

impl FrameGraphBuilder {
    fn add_resource() {

    }

    fn add_pass() {

    }
}

struct ResourceBuilder {

}

impl ResourceBuilder {
    fn build(frame_graph_builder: &FrameGraphBuilder) {

    }
}

struct PassBuilder {

}

impl PassBuilder {
    fn build(frame_graph_builder: &FrameGraphBuilder) {

    }
}

struct PassModuleInput {

}

struct PassModuleOutput {
    diffuse: Surface
}

fn set_up_pass_module(input: PassModuleInput, output: PassModuleOutput) {

}


fn main() {

    let swapchain_backbuffer = frame_builder.add_external_resource();


    let graphics_queue = frame_builder.get_queue(QueueType::Graphics);

    frame_builder.add_pass(graphics_queue)
        .reads_resource(swapchain_backbuffer)
        .writes_resource(swapchain_backbuffer);



    //strategy 1: writing to a resource consumes the ID and produces a new ID
    //strategy 2: explicitly say if A is before or after B
    //strategy 3: fence resource

    let backbuffer = frame_builder.create_buffer();
    let pass1 = frame_builder.add_pass(graphics_queue);

    pass1.reads_resource(swapchain_backbuffer);
    let swapchain_backbuffer = pass1.writes_resources(swapchain_backbuffer);

    // awareness of pipelines?
    // non-intrusive insertion of new logic?
    // unify cpu and gpu scheduling?

}

trait Pass {
    fn requirements(requirement: &mut PassRequirements) {
        requirement.read(asdf);
        requirement.read(asdf);
        requirement.write(asdf);

        requirement.after_fence(asdf);

        requirement.before_fence(asdf);
    }
}

*/

// * Simulate
//   - Static visibility can be calculated in parallel
// * Compute Views
//   - First person camera, shadow maps, etc.
// * View Visibility Job(s)
//   - Dynamic visibility, merge with results from static visibility
// * Populate View Render Nodes
// * Extract Per View Job(s)
// * Extract Finish
// * Prepare Per View Job(s)
// * Publish to Submit
// * High-Level Submit Script Job(s)
// * Submit View Job(s)
// * End Frame Job
// * Submit Done

// view subscribes to stages
// objects subscribe to stages

// Example:
// - Shadow casting objects subscribe to shadow generate stage
// - Shadow view subscribes to shadow generate stage

// A material defines the stages/shaders per stage
// -

// script includes target bind, clear, resolve

// Generate command buffers

// Submit them later

// destiny example
/*
fn generate_gbuffer_pass() {
    set_render_targets(depth_stencil, gbuffer_surfaces);
    setup_viewport_parameters();
    // ...
    clear_viewport();
    submit_render_stage_for_view(first_person_view, _render_stage_gbuffer_opaque);
}
*/

// User calls functions to register render objects
// - This is a retained API because render object existence loads streaming assets
// User calls functions to register visibility objects
// - This is a retained API because presumably we don't want to rebuild spatial structures every frame
// Take input
// User calls function to create FPS view
// User could call function to calculate visibility of static objects for FPS camera early to reduce
//   future critical-path work (to reduce latency)
// Simulation
// User calls functions to create more views (such as shadows, minimap, etc.)
// User calls functions to start jobs that calculate dynamic visibility for FPS view
// User calls functions to start jobs that calculate static and dynamic visibility for all other views
// After these jobs end, user calls functions to start jobs that extract data
// User calls function to kick off the prepare/submit pipeline
// Return to the top...
// - The render pipeline uses cached data, node, static data. So it can run concurrently with all above steps
// - The visibility spatial structures and render objects can be added/removed freely during the above steps

use renderer::visibility::*;
use renderer::features::sprite::*;
use renderer::features::static_quad::*;
use renderer::phases::draw_opaque::*;
use renderer::{RenderPhaseMaskBuilder, FramePacketBuilder, ExtractJobSet, AllRenderNodes};
use renderer::RenderRegistry;
use renderer::RenderViewSet;
use legion::prelude::*;
use glam::Vec3;

#[derive(Copy, Clone)]
struct PositionComponent {
    position: Vec3,
}

#[derive(Clone)]
struct SpriteComponent {
    sprite_handle: SpriteRenderNodeHandle,
    visibility_handle: DynamicAabbVisibilityNodeHandle,
}

fn main() {
    // Setup logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    //
    // Setup render features
    //
    RenderRegistry::register_feature::<SpriteRenderFeature>();
    RenderRegistry::register_feature::<StaticQuadRenderFeature>();
    RenderRegistry::register_render_phase::<DrawOpaqueRenderPhase>();

    let main_camera_render_phase_mask = RenderPhaseMaskBuilder::default()
        .add_render_phase::<DrawOpaqueRenderPhase>()
        .build();

    let minimap_render_phase_mask = RenderPhaseMaskBuilder::default()
        .add_render_phase::<DrawOpaqueRenderPhase>()
        .build();

    // In theory we could pre-cook static visibility in chunks and stream them in
    let static_visibility_node_set = StaticVisibilityNodeSet::default();
    let mut dynamic_visibility_node_set = DynamicVisibilityNodeSet::default();
    let mut sprite_render_nodes = SpriteRenderNodeSet::new();

    //
    // Init an example world state
    //
    let universe = Universe::default();
    let mut world = universe.create_world();

    let sprites = ["sprite1", "sprite2", "sprite3"];

    for i in 0..100 {
        let position = Vec3::new(((i / 10) * 100) as f32, ((i % 10) * 100) as f32, 0.0);
        let _sprite = sprites[i % sprites.len()];

        //TODO: Not clear the best approach from an API perspective to allocate component and render
        // node that point at each other. (We can't get Entity or Handle until the object is inserted)
        let sprite_info = SpriteRenderNode {
            // entity handle
            // sprite asset
        };

        // User calls functions to register render objects
        // - This is a retained API because render object existence loads streaming assets
        let sprite_handle = sprite_render_nodes.register_sprite(sprite_info);

        let aabb_info = DynamicAabbVisibilityNode {
            // render node handles
            // aabb bounds
            handle: sprite_handle.into(),
        };

        // User calls functions to register visibility objects
        // - This is a retained API because presumably we don't want to rebuild spatial structures every frame
        let visibility_handle = dynamic_visibility_node_set.register_dynamic_aabb(aabb_info);

        let position_component = PositionComponent { position };
        let sprite_component = SpriteComponent {
            sprite_handle,
            visibility_handle,
        };

        world.insert(
            (),
            (0..1).map(|_| (position_component, sprite_component.clone())),
        );
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

        // User calls function to create "main" view
        let view = glam::Mat4::look_at_rh(
            glam::Vec3::from([0.0, 0.0, 5.0]),
            glam::Vec3::from([0.0, 0.0, 0.0]),
            glam::Vec3::from([0.0, 1.0, 0.0]),
        );

        let frustum_width = 800;
        let frustum_height = 600;
        let projection = glam::Mat4::orthographic_rh(
            0.0,
            frustum_width as f32,
            0.0,
            frustum_height as f32,
            100.0,
            -100.0,
        );

        let view_proj = projection * view;

        let main_view = render_view_set.create_view(
            view_proj,
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
        //TODO: Moving an object would require updating visibility nodes (likely a remove and re-insert)

        //
        // Figure out other views (example: minimap, shadow maps, etc.)
        //
        let minimap_view = render_view_set.create_view(
            view_proj,
            minimap_render_phase_mask,
            "minimap".to_string(),
        );

        //
        // Finish visibility calculations and populate the frame packet. Views can potentially be run in their own jobs
        // in the future. The visibility calculations and allocation of frame packet nodes can all run in parallel.
        // There is a single sync point after this to give features a callback that extraction is about to begin.
        // (All frame nodes must be allocated before this point). After that, extraction for all features and views
        // can run in parallel.
        //
        // I'm not sure why a pre-extract callback that can access all frame nodes is useful but it was called out
        // in the bungie talk so implementing it for now. Removing this would allow extraction to move forward for
        // views that finish visibility without waiting on visibility for other views
        //

        // The frame packet builder will merge visibility results and hold extracted data from simulation. During
        // the extract window, render nodes cannot be added/removed
        let mut all_render_nodes = AllRenderNodes::new();
        all_render_nodes.add_render_nodes(&sprite_render_nodes);

        let frame_packet_builder = FramePacketBuilder::new(&all_render_nodes);

        // User calls functions to start jobs that calculate dynamic visibility for FPS view
        let main_view_dynamic_visibility_result =
            dynamic_visibility_node_set.calculate_dynamic_visibility(&main_view);

        // User calls functions to start jobs that calculate static and dynamic visibility for all other views
        let minimap_static_visibility_result =
            static_visibility_node_set.calculate_static_visibility(&minimap_view);
        let minimap_dynamic_visibility_result =
            dynamic_visibility_node_set.calculate_dynamic_visibility(&minimap_view);

        log::info!(
            "main view static node count: {}",
            main_view_static_visibility_result.handles.len()
        );
        log::info!(
            "main view dynamic node count: {}",
            main_view_dynamic_visibility_result.handles.len()
        );

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
        // extract_impl_set.add_impl(Box::new(SpriteRenderFeature));
        // extract_impl_set.add_impl(Box::new(StaticQuadRenderFeature));

        let frame_packet = frame_packet_builder.build();
        println!("frame packet:\n{:#?}", frame_packet);

        let mut extract_job_set = ExtractJobSet::new();
        extract_job_set.add_job(Box::new(SpriteExtractJob::new()));
        extract_job_set.add_job(Box::new(StaticQuadExtractJob::new()));

        // let new_world = universe.create_world();
        // world.merge(new_world);

        let prepare_job_set =
            extract_job_set.extract(&world, &frame_packet, &[&main_view, &minimap_view]);

        //extract_job_set.extract(&frame_packet, &[&main_view, &minimap_view]);

        //
        // At this point, we can start the next simulation loop. The renderer has everything it needs
        // to render the game without referring to game state stored in the frame packet or feature renderers.
        // Visibility and render nodes can be modified up to the point that we start doing visibility
        // checks and building the next frame packet
        //
        let _submit_job = prepare_job_set.prepare();

        //render_feature_set.prepare(&frame_packet, &[&main_view, &minimap_view]);
        //render_feature_set.submit(&frame_packet, &[&main_view, &minimap_view]);

        // User calls function to kick off the prepare/submit pipeline
        //render_node_set.prepare(&frame_packet);
        //render_node_set.submit(&frame_packet);
    }

    //
    // Unregister render nodes/visibility objects
    //
    let query = <Read<SpriteComponent>>::query();
    for sprite_component in query.iter(&mut world) {
        sprite_render_nodes.unregister_sprite(sprite_component.sprite_handle);
        dynamic_visibility_node_set.unregister_dynamic_aabb(sprite_component.visibility_handle);
    }
}

//TODO:
// - Render graph of non-transparents and then transparents
// - maybe add 2d lighting?
// - Add support for streaming chunks?
// - tilemap?

// Create views
// Calculate static visibility (jobify)
// Calculate dynamic visibility (jobify)
// Extract data
// Prepare
// Submit
