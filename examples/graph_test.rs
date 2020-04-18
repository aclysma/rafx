use renderer::visibility::*;
use renderer::features::sprite::*;
use renderer::features::static_quad::*;
use renderer::phases::draw_opaque::*;
use renderer::{RenderPhaseMaskBuilder, FramePacketBuilder, ExtractJobSet, AllRenderNodes};
use renderer::RenderRegistryBuilder;
use renderer::RenderViewSet;
use legion::prelude::*;
use glam::Vec3;
use renderer_ext::{ExtractSource, PositionComponent, SpriteComponent, CommandWriter};
use renderer_ext::phases::draw_transparent::DrawTransparentRenderPhase;

fn main() {
    // Setup logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    //
    // Setup render features
    //
    let render_registry = RenderRegistryBuilder::default()
        .register_feature::<SpriteRenderFeature>()
        .register_feature::<StaticQuadRenderFeature>()
        .register_render_phase::<DrawOpaqueRenderPhase>()
        .register_render_phase::<DrawTransparentRenderPhase>()
        .build();

    let main_camera_render_phase_mask = RenderPhaseMaskBuilder::default()
        .add_render_phase::<DrawOpaqueRenderPhase>()
        .add_render_phase::<DrawTransparentRenderPhase>()
        .build();

    let minimap_render_phase_mask = RenderPhaseMaskBuilder::default()
        .add_render_phase::<DrawOpaqueRenderPhase>()
        .add_render_phase::<DrawTransparentRenderPhase>()
        .build();

    // In theory we could pre-cook static visibility in chunks and stream them in
    let static_visibility_node_set = StaticVisibilityNodeSet::default();
    let mut dynamic_visibility_node_set = DynamicVisibilityNodeSet::default();
    let sprite_render_nodes = SpriteRenderNodeSet::new();

    //
    // Init an example world state
    //
    let universe = Universe::new();
    let mut world = universe.create_world();
    let mut resources = legion::systems::resource::Resources::default();

    resources.insert(sprite_render_nodes);

    {
        let sprites = ["sprite1", "sprite2", "sprite3"];
        for i in 0..100 {
            let position = Vec3::new(((i / 10) * 100) as f32, ((i % 10) * 100) as f32, 0.0);
            let alpha = if i % 7 == 0 { 0.50 } else { 1.0 };
            let _sprite = sprites[i % sprites.len()];

            let mut sprite_render_nodes = resources.get_mut::<SpriteRenderNodeSet>().unwrap();

            // User calls functions to register render objects
            // - This is a retained API because render object existence can trigger loading streaming assets and
            //   keep them resident in memory
            // - Some render objects might not correspond to legion entities, and some people might not be using
            //   legion at all
            // - the `_with_handle` variant allows us to get the handle of the value that's going to be allocated
            //   This resolves a circular dependency where the component needs the render node handle and the
            //   render node needs the entity.
            sprite_render_nodes.register_sprite_with_handle(|sprite_handle| {
                let aabb_info = DynamicAabbVisibilityNode {
                    handle: sprite_handle.into(),
                    // aabb bounds
                };

                // User calls functions to register visibility objects
                // - This is a retained API because presumably we don't want to rebuild spatial structures every frame
                let visibility_handle =
                    dynamic_visibility_node_set.register_dynamic_aabb(aabb_info);

                let position_component = PositionComponent { position };
                let sprite_component = SpriteComponent {
                    sprite_handle,
                    visibility_handle,
                    alpha,
                };

                let entity = world.insert(
                    (),
                    (0..1).map(|_| (position_component, sprite_component.clone())),
                )[0];

                println!("create entity {:?}", entity);
                world.get_component::<PositionComponent>(entity).unwrap();

                SpriteRenderNode {
                    entity, // sprite asset
                }
            });
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
        let projection = glam::Mat4::orthographic_rh(
            0.0,
            frustum_width as f32,
            0.0,
            frustum_height as f32,
            100.0,
            -100.0,
        );

        let view_proj = projection * view;

        println!("eye is at {}", view_proj);

        let main_view = render_view_set.create_view(
            eye_position,
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
            eye_position,
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

        let sprite_render_nodes = resources.get::<SpriteRenderNodeSet>().unwrap();
        let mut all_render_nodes = AllRenderNodes::new();
        all_render_nodes.add_render_nodes(&*sprite_render_nodes);

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
        //println!("frame packet:\n{:#?}", frame_packet);

        let prepare_job_set = {
            let mut extract_job_set = ExtractJobSet::new();
            extract_job_set.add_job(create_sprite_extract_job());
            extract_job_set.add_job(create_static_quad_extract_job());

            let extract_source = ExtractSource::new(&world, &resources);
            extract_job_set.extract(&extract_source, &frame_packet, &[&main_view, &minimap_view])
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
        let prepared_render_data = prepare_job_set.prepare(
            &frame_packet,
            &[&main_view, &minimap_view],
            &render_registry,
        );

        // At this point the end-user can kick off the final write job per view/phase pair. The
        // output of this is left up to the end user and would likely be something like a GPU
        // command buffer.
        let mut write_context = CommandWriter {};
        prepared_render_data
            .write_view_phase::<DrawOpaqueRenderPhase>(&main_view, &mut write_context);
        prepared_render_data
            .write_view_phase::<DrawTransparentRenderPhase>(&main_view, &mut write_context);
        prepared_render_data
            .write_view_phase::<DrawOpaqueRenderPhase>(&minimap_view, &mut write_context);
        prepared_render_data
            .write_view_phase::<DrawTransparentRenderPhase>(&minimap_view, &mut write_context);
    }

    //
    // Unregister render nodes/visibility objects
    //
    let mut sprite_render_nodes = resources.get_mut::<SpriteRenderNodeSet>().unwrap();
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
