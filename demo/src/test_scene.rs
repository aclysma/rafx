use legion::{Resources, World};
use crate::asset_resource::AssetResource;
use glam::f32::Vec3;
use crate::features::sprite::{SpriteRenderNodeSet, SpriteRenderNode};
use renderer::visibility::{DynamicVisibilityNodeSet, DynamicAabbVisibilityNode};
use crate::components::{
    PositionComponent, SpriteComponent, PointLightComponent, SpotLightComponent,
    DirectionalLightComponent,
};
use crate::features::mesh::{MeshRenderNodeSet, MeshRenderNode};
use atelier_assets::core::asset_uuid;
use atelier_assets::core as atelier_core;
use crate::components::MeshComponent;
use renderer::assets::ImageAsset;
use crate::game_asset_lookup::MeshAsset;

pub fn populate_test_sprite_entities(
    resources: &mut Resources,
    world: &mut World,
) {
    let sprite_image = {
        let asset_resource = resources.get::<AssetResource>().unwrap();
        asset_resource.load_asset::<ImageAsset>(asset_uuid!("7c42f3bc-e96b-49f6-961b-5bfc799dee50"))
    };

    for i in 0..1000 {
        let position = Vec3::new(((i / 10) * 25) as f32, ((i % 10) * 25) as f32, 0.0);
        //let alpha = if i % 7 == 0 { 0.50 } else { 1.0 };
        let alpha = 1.0;

        let mut sprite_render_nodes = resources.get_mut::<SpriteRenderNodeSet>().unwrap();
        let mut dynamic_visibility_node_set =
            resources.get_mut::<DynamicVisibilityNodeSet>().unwrap();

        let render_node = sprite_render_nodes.register_sprite(SpriteRenderNode {
            position,
            alpha,
            image: sprite_image.clone(),
        });

        let aabb_info = DynamicAabbVisibilityNode {
            handle: render_node.as_raw_generic_handle(),
            // aabb bounds
        };

        // User calls functions to register visibility objects
        // - This is a retained API because presumably we don't want to rebuild spatial structures every frame
        let visibility_node = dynamic_visibility_node_set.register_dynamic_aabb(aabb_info);

        let position_component = PositionComponent { position };
        let sprite_component = SpriteComponent {
            render_node,
            visibility_node,
            alpha,
            image: sprite_image.clone(),
        };

        world.extend((0..1).map(|_| (position_component, sprite_component.clone())));
    }
}

pub fn populate_test_mesh_entities(
    resources: &mut Resources,
    world: &mut World,
) {
    let mut mesh_render_nodes = resources.get_mut::<MeshRenderNodeSet>().unwrap();
    let mut dynamic_visibility_node_set = resources.get_mut::<DynamicVisibilityNodeSet>().unwrap();

    //
    // Add a floor
    //
    {
        let floor_mesh = {
            let asset_resource = resources.get::<AssetResource>().unwrap();
            asset_resource
                .load_asset::<MeshAsset>(asset_uuid!("f355d620-2971-48d5-b5c5-2fc9cf254525"))
        };

        let position = Vec3::new(0.0, 0.0, -1.0);

        let render_node = mesh_render_nodes.register_mesh(MeshRenderNode {
            transform: glam::Mat4::from_translation(position),
            mesh: Some(floor_mesh.clone()),
        });

        // User calls functions to register visibility objects
        // - This is a retained API because presumably we don't want to rebuild spatial structures every frame
        let visibility_node =
            dynamic_visibility_node_set.register_dynamic_aabb(DynamicAabbVisibilityNode {
                handle: render_node.as_raw_generic_handle(),
                // aabb bounds
            });

        let position_component = PositionComponent { position };
        let mesh_component = MeshComponent {
            render_node,
            visibility_node,
            mesh: Some(floor_mesh.clone()),
        };

        world.extend((0..1).map(|_| (position_component, mesh_component.clone())));
    }

    //
    // Add some cubes
    //
    {
        let cube_meshes = {
            let asset_resource = resources.get::<AssetResource>().unwrap();
            let mut meshes = Vec::default();

            // container1
            meshes.push(
                asset_resource
                    .load_asset::<MeshAsset>(asset_uuid!("9a513889-c0bd-45e8-9c70-d5388fd0bb5a")),
            );

            // container2
            meshes.push(
                asset_resource
                    .load_asset::<MeshAsset>(asset_uuid!("07b7319f-199c-416f-87e2-414649797fe9")),
            );

            // blue icosphere
            meshes.push(
                asset_resource
                    .load_asset::<MeshAsset>(asset_uuid!("8a51f05c-f3c6-4d61-bc9d-16c2337265f1")),
            );

            meshes
        };

        for i in 0..6 {
            let position = Vec3::new(((i / 3) * 3) as f32, ((i % 3) * 3) as f32, 0.0);
            let cube_mesh = cube_meshes[i % cube_meshes.len()].clone();

            let render_node = mesh_render_nodes.register_mesh(MeshRenderNode {
                transform: glam::Mat4::from_translation(position),
                mesh: Some(cube_mesh.clone()),
            });

            // User calls functions to register visibility objects
            // - This is a retained API because presumably we don't want to rebuild spatial structures every frame
            let visibility_node =
                dynamic_visibility_node_set.register_dynamic_aabb(DynamicAabbVisibilityNode {
                    handle: render_node.as_raw_generic_handle(),
                    // aabb bounds
                });

            let position_component = PositionComponent { position };
            let mesh_component = MeshComponent {
                render_node,
                visibility_node,
                mesh: Some(cube_mesh.clone()),
            };

            world.extend((0..1).map(|_| (position_component, mesh_component.clone())));
        }
    }
}

pub fn populate_test_lights(
    resources: &mut Resources,
    world: &mut World,
) {
    // add_point_light(
    //     resources,
    //     world,
    //     glam::Vec3::new(-3.0, -3.0, 3.0),
    //     PointLightComponent {
    //         color: [1.0, 1.0, 1.0, 1.0].into(),
    //         intensity: 130.0,
    //         range: 25.0,
    //     },
    // );
    //
    // add_point_light(
    //     resources,
    //     world,
    //     glam::Vec3::new(-3.0, 3.0, 3.0),
    //     PointLightComponent {
    //         color: [1.0, 1.0, 1.0, 1.0].into(),
    //         intensity: 130.0,
    //         range: 25.0,
    //     },
    // );

    // let light_from = glam::Vec3::new(-3.0, -3.0, 0.0);
    // let light_to = glam::Vec3::zero();
    // let light_direction = (light_to - light_from).normalize();
    // add_spot_light(
    //     resources,
    //     world,
    //     light_from,
    //     SpotLightComponent {
    //         direction: light_direction,
    //         spotlight_half_angle: 10.0 * (std::f32::consts::PI / 180.0),
    //         range: 8.0,
    //         color: [1.0, 1.0, 1.0, 1.0].into(),
    //         intensity: 1000.0,
    //     },
    // );

    let light_from = glam::Vec3::new(5.0, 5.0, 5.0);
    let light_to = glam::Vec3::zero();
    let light_direction = (light_to - light_from).normalize();
    add_directional_light(
        resources,
        world,
        DirectionalLightComponent {
            direction: light_direction,
            intensity: 8.0,
            color: [1.0, 1.0, 1.0, 1.0].into(),
        },
    );
}

fn add_directional_light(
    _resources: &mut Resources,
    world: &mut World,
    light_component: DirectionalLightComponent,
) {
    world.extend(vec![(light_component,)]);
}

fn add_spot_light(
    _resources: &mut Resources,
    world: &mut World,
    position: glam::Vec3,
    light_component: SpotLightComponent,
) {
    let position_component = PositionComponent { position };

    world.extend(vec![(position_component, light_component)]);
}

fn add_point_light(
    _resources: &mut Resources,
    world: &mut World,
    position: glam::Vec3,
    light_component: PointLightComponent,
) {
    let position_component = PositionComponent { position };

    world.extend(vec![(position_component, light_component)]);
}
