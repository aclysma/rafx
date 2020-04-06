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










use renderer::StaticVisibilityNodeSet;
use renderer::DynamicVisibilityNodeSet;
use renderer::RenderNodeSet;
use renderer::DynamicAabbVisibilityNode;
use renderer::SpriteRenderNode;
use renderer::RenderView;
use renderer::FramePacket;

fn main() {
    // Could maybe have multiple of these? Could pre-cook and serialize?
    let mut static_visibility_node_set = StaticVisibilityNodeSet::default();
    let mut dynamic_visibility_node_set = DynamicVisibilityNodeSet::default();
    let mut render_node_set = RenderNodeSet::default();

    let sprite_info = SpriteRenderNode {
        // entity handle
        // sprite asset
    };

    // User calls functions to register render objects
    // - This is a retained API because render object existence loads streaming assets
    let sprite_handle = render_node_set.register_sprite(sprite_info);

    let aabb_info = DynamicAabbVisibilityNode {
        // render node handles
        // aabb bounds
        handle: sprite_handle.into()
    };

    // User calls functions to register visibility objects
    // - This is a retained API because presumably we don't want to rebuild spatial structures every frame
    let aabb_handle = dynamic_visibility_node_set.register_dynamic_aabb(aabb_info);

    // Would we need to update these? i.e. visibility_node_set.move_aabb()?

    for _ in 0..100 {
        println!("----- FRAME -----");

        // Take input

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

        let mut frame_packet = FramePacket::default();
        let main_view = RenderView::new(&mut frame_packet, view_proj/*, which_passes_are_enabled*/);

        //TODO: Separate static/dynamic visibility node sets? Do the static updates need to happen here before calculating static vis?

        // User could call function to calculate visibility of static objects for FPS camera early to reduce
        //   future critical-path work (to reduce latency)
        let main_view_static_visibility_result = static_visibility_node_set.calculate_static_visibility(&main_view); // return task?

        // Simulation

        // User calls functions to create more views (such as shadows, minimap, etc.) based on simulation results
        let minimap_view = RenderView::new(&mut frame_packet, view_proj/*, which_passes_are_enabled*/);

        // User calls functions to start jobs that calculate dynamic visibility for FPS view
        let main_view_dynamic_visibility_result = dynamic_visibility_node_set.calculate_dynamic_visibility(&main_view); // return task?

        // User calls functions to start jobs that calculate static and dynamic visibility for all other views
        let minimap_static_visibility_result = static_visibility_node_set.calculate_static_visibility(&minimap_view);
        let minimap_dynamic_visibility_result = dynamic_visibility_node_set.calculate_dynamic_visibility(&minimap_view);

        println!("static nodes: {}", main_view_static_visibility_result.handles.len());
        println!("dynamic nodes: {}", main_view_dynamic_visibility_result.handles.len());

        // After these jobs end, user calls functions to start jobs that extract data
        main_view.allocate_frame_packet_nodes(
            &render_node_set,
            &mut frame_packet,
            &main_view_static_visibility_result,
            &main_view_dynamic_visibility_result
        );

        main_view.extract(
            &mut frame_packet,
            //&world
        );

        minimap_view.allocate_frame_packet_nodes(
            &render_node_set,
            &mut frame_packet,
            &main_view_static_visibility_result,
            &main_view_dynamic_visibility_result
        );

        minimap_view.extract(
            &mut frame_packet,
            //&world
        );

        // Join Extract Jobs. At this point the frame packet and view packets are read-only. Simulation can continue.

        // User calls function to kick off the prepare/submit pipeline
        render_node_set.prepare(&mut frame_packet);
        render_node_set.submit(&mut frame_packet);


        // Return to the top...
        // - The render pipeline uses cached data, node, static data. So it can run concurrently with all above steps
        // - The visibility spatial structures and render objects can be added/removed freely during the above steps




    }




    //TODO:
    // - Render graph of non-transparents and then transparents
    // - maybe add 2d lighting?
    // - Add support for streaming chunks?
    // - tilemap?


    render_node_set.unregister_sprite(sprite_handle);
    dynamic_visibility_node_set.unregister_dynamic_aabb(aabb_handle);
}



// Create views
// Calculate static visibility (jobify)
// Calculate dynamic visibility (jobify)
// Extract data
// Prepare
// Submit