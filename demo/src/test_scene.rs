use crate::asset_resource::AssetResource;
use crate::components::MeshComponent;
use crate::components::{
    DirectionalLightComponent, PointLightComponent, PositionComponent, SpotLightComponent,
    SpriteComponent,
};
use crate::features::mesh::{MeshRenderNode, MeshRenderNodeSet};
use crate::features::sprite::{SpriteRenderNode, SpriteRenderNodeSet};
use crate::game_asset_lookup::MeshAsset;
use atelier_assets::core as atelier_core;
use atelier_assets::core::asset_uuid;
use glam::f32::Vec3;
use legion::{Resources, World};
use rafx::assets::ImageAsset;
use rafx::visibility::{DynamicAabbVisibilityNode, DynamicVisibilityNodeSet};

pub fn populate_test_sprite_entities(
    resources: &mut Resources,
    world: &mut World,
) {
    let sprite_image = {
        let asset_resource = resources.get::<AssetResource>().unwrap();
        //asset_resource.load_asset_path::<ImageAsset, _>("textures/texture2.jpg")
        asset_resource.load_asset::<ImageAsset>(asset_uuid!("cad0eeb3-68e1-48a5-81b6-ba4a7e848f38"))
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
            asset_resource.load_asset_path::<MeshAsset, _>("blender/cement_floor.glb")
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
                asset_resource.load_asset_path("blender/storage_container1.glb"),
                //.load_asset::<MeshAsset>(asset_uuid!("b461ed48-d2f8-44af-bcda-c5b64633c13d")),
            );

            // container2
            meshes.push(
                asset_resource.load_asset_path("blender/storage_container2.glb"),
                //.load_asset::<MeshAsset>(asset_uuid!("04ea64c6-d4da-4ace-83e7-56f4d60524c1")),
            );

            // blue icosphere - load by UUID since it's one of several meshes in the file
            meshes.push(
                asset_resource
                    .load_asset::<MeshAsset>(asset_uuid!("d5aed900-1e31-4f47-94ba-e356b0b0b8b0")),
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
    //populate_test_lights_shadow_acne(resources, world);

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
    add_point_light(
        resources,
        world,
        //glam::Vec3::new(-3.0, 3.0, 2.0),
        glam::Vec3::new(5.0, 5.0, 2.0),
        PointLightComponent {
            color: [0.0, 1.0, 0.0, 1.0].into(),
            intensity: 50.0,
            range: 25.0,
        },
    );

    //
    // KEY LIGHT
    //
    // let light_from = glam::Vec3::new(5.0, 5.0, 5.0);
    // let light_to = glam::Vec3::zero();
    // let light_direction = (light_to - light_from).normalize();
    // add_directional_light(
    //     resources,
    //     world,
    //     DirectionalLightComponent {
    //         direction: light_direction,
    //         intensity: 1.0,
    //         color: [1.0, 1.0, 1.0, 1.0].into(),
    //     },
    // );
    //
    // //
    // // KEY LIGHT
    // //
    let light_from = glam::Vec3::new(-5.0, 5.0, 5.0);
    let light_to = glam::Vec3::zero();
    let light_direction = (light_to - light_from).normalize();
    add_directional_light(
        resources,
        world,
        DirectionalLightComponent {
            direction: light_direction,
            intensity: 1.0,
            color: [0.0, 0.0, 1.0, 1.0].into(),
        },
    );

    //
    // KEY LIGHT
    //
    // let light_from = glam::Vec3::new(0.0, -7.0, 5.0);
    // let light_to = glam::Vec3::zero();
    // let light_direction = (light_to - light_from).normalize();
    // add_directional_light(
    //     resources,
    //     world,
    //     DirectionalLightComponent {
    //         direction: light_direction,
    //         intensity: 2.0,
    //         color: [0.0, 0.0, 1.0, 1.0].into(),
    //     },
    // );

    //
    // SPOT LIGHT
    //
    let light_from = glam::Vec3::new(-3.0, -3.0, 5.0);
    let light_to = glam::Vec3::zero();
    let light_direction = (light_to - light_from).normalize();
    add_spot_light(
        resources,
        world,
        light_from,
        SpotLightComponent {
            direction: light_direction,
            spotlight_half_angle: 40.0 * (std::f32::consts::PI / 180.0),
            range: 12.0,
            color: [1.0, 0.0, 0.0, 1.0].into(),
            intensity: 500.0,
        },
    );
}

pub fn populate_test_lights_shadow_acne(
    resources: &mut Resources,
    world: &mut World,
) {
    //
    // ALMOST VERTICAL (SHADOW ACNE TEST)
    //
    let light_from = glam::Vec3::new(3.0, 3.0, 25.0);
    let light_to = glam::Vec3::zero();
    let light_direction = (light_to - light_from).normalize();
    add_directional_light(
        resources,
        world,
        DirectionalLightComponent {
            direction: light_direction,
            intensity: 2.0,
            color: [1.0, 1.0, 1.0, 1.0].into(),
        },
    );

    //
    // ALMOST HORIZONTAL (SHADOW ACNE TEST)
    //
    let light_from = glam::Vec3::new(-5.0, 5.0, 0.2);
    let light_to = glam::Vec3::zero();
    let light_direction = (light_to - light_from).normalize();
    add_directional_light(
        resources,
        world,
        DirectionalLightComponent {
            direction: light_direction,
            intensity: 25.0,
            color: [1.0, 1.0, 1.0, 1.0].into(),
        },
    );

    //
    // 45 degree (SHADOW ACNE TEST)
    //
    let light_from = glam::Vec3::new(-5.0, 5.0, 5.0);
    let light_to = glam::Vec3::zero();
    let light_direction = (light_to - light_from).normalize();
    add_directional_light(
        resources,
        world,
        DirectionalLightComponent {
            direction: light_direction,
            intensity: 1.0,
            color: [1.0, 1.0, 1.0, 1.0].into(),
        },
    );

    //
    // SPOT LIGHT
    //
    let light_from = glam::Vec3::new(-3.0, -3.0, 5.0);
    let light_to = glam::Vec3::zero();
    let light_direction = (light_to - light_from).normalize();
    add_spot_light(
        resources,
        world,
        light_from,
        SpotLightComponent {
            direction: light_direction,
            spotlight_half_angle: 40.0 * (std::f32::consts::PI / 180.0),
            range: 12.0,
            color: [1.0, 0.0, 0.0, 1.0].into(),
            intensity: 500.0,
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
