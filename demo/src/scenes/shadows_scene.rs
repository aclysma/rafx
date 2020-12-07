use crate::asset_resource::AssetResource;
use crate::components::{DirectionalLightComponent, PointLightComponent, PositionComponent};
use crate::components::{MeshComponent, SpotLightComponent};
use crate::features::mesh::{MeshRenderNode, MeshRenderNodeSet};
use crate::game_asset_lookup::MeshAsset;
use crate::time::TimeState;
use atelier_assets::core as atelier_core;
use atelier_assets::core::asset_uuid;
use glam::f32::Vec3;
use legion::IntoQuery;
use legion::{Read, Resources, World, Write};
use rafx::visibility::{DynamicAabbVisibilityNode, DynamicVisibilityNodeSet};

pub(super) struct ShadowsScene {}

impl ShadowsScene {
    pub(super) fn new(
        world: &mut World,
        resources: &Resources,
    ) -> Self {
        let mut mesh_render_nodes = resources.get_mut::<MeshRenderNodeSet>().unwrap();
        let mut dynamic_visibility_node_set =
            resources.get_mut::<DynamicVisibilityNodeSet>().unwrap();

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
        // Add some meshes
        //
        {
            let cube_meshes =
                {
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
                    meshes.push(asset_resource.load_asset::<MeshAsset>(asset_uuid!(
                        "d5aed900-1e31-4f47-94ba-e356b0b0b8b0"
                    )));

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

        //
        // POINT LIGHT
        //
        super::add_point_light(
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
        // DIRECTIONAL LIGHT
        //
        let light_from = glam::Vec3::new(-5.0, 5.0, 5.0);
        let light_to = glam::Vec3::zero();
        let light_direction = (light_to - light_from).normalize();
        super::add_directional_light(
            resources,
            world,
            DirectionalLightComponent {
                direction: light_direction,
                intensity: 1.0,
                color: [0.0, 0.0, 1.0, 1.0].into(),
            },
        );

        //
        // SPOT LIGHT
        //
        let light_from = glam::Vec3::new(-3.0, -3.0, 5.0);
        let light_to = glam::Vec3::zero();
        let light_direction = (light_to - light_from).normalize();
        super::add_spot_light(
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

        ShadowsScene {}
    }
}

impl super::TestScene for ShadowsScene {
    fn update(
        &mut self,
        world: &mut World,
        resources: &Resources,
    ) {
        super::add_light_debug_draw(&resources, &world);

        {
            let time_state = resources.get::<TimeState>().unwrap();
            let mut query = <Write<DirectionalLightComponent>>::query();
            for mut light in query.iter_mut(world) {
                const LIGHT_XY_DISTANCE: f32 = 50.0;
                const LIGHT_Z: f32 = 50.0;
                const LIGHT_ROTATE_SPEED: f32 = 0.0;
                const LIGHT_LOOP_OFFSET: f32 = 2.0;
                let loop_time = time_state.total_time().as_secs_f32();
                let light_from = glam::Vec3::new(
                    LIGHT_XY_DISTANCE
                        * f32::cos(LIGHT_ROTATE_SPEED * loop_time + LIGHT_LOOP_OFFSET),
                    LIGHT_XY_DISTANCE
                        * f32::sin(LIGHT_ROTATE_SPEED * loop_time + LIGHT_LOOP_OFFSET),
                    LIGHT_Z,
                    //LIGHT_Z// * f32::sin(LIGHT_ROTATE_SPEED * loop_time + LIGHT_LOOP_OFFSET).abs(),
                    //0.2
                    //2.0
                );
                let light_to = glam::Vec3::default();

                light.direction = (light_to - light_from).normalize();
            }
        }

        {
            let time_state = resources.get::<TimeState>().unwrap();
            let mut query = <(Write<PositionComponent>, Read<PointLightComponent>)>::query();
            for (position, _light) in query.iter_mut(world) {
                const LIGHT_XY_DISTANCE: f32 = 6.0;
                const LIGHT_Z: f32 = 3.5;
                const LIGHT_ROTATE_SPEED: f32 = 0.5;
                const LIGHT_LOOP_OFFSET: f32 = 2.0;
                let loop_time = time_state.total_time().as_secs_f32();
                let light_from = glam::Vec3::new(
                    LIGHT_XY_DISTANCE
                        * f32::cos(LIGHT_ROTATE_SPEED * loop_time + LIGHT_LOOP_OFFSET),
                    LIGHT_XY_DISTANCE
                        * f32::sin(LIGHT_ROTATE_SPEED * loop_time + LIGHT_LOOP_OFFSET),
                    LIGHT_Z,
                    //LIGHT_Z// * f32::sin(LIGHT_ROTATE_SPEED * loop_time + LIGHT_LOOP_OFFSET).abs(),
                    //0.2
                    //2.0
                );
                position.position = light_from;
            }
        }
    }
}
