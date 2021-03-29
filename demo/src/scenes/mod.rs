use crate::components::{
    DirectionalLightComponent, PointLightComponent, PositionComponent, SpotLightComponent,
};
use crate::features::debug3d::DebugDraw3DResource;
use legion::IntoQuery;
use legion::{Read, Resources, World};

mod shadows_scene;
use shadows_scene::ShadowsScene;

mod sprite_scene;
use sprite_scene::SpriteScene;

#[derive(Copy, Clone, Debug)]
pub enum Scene {
    Shadows,
    Sprite,
}

pub const ALL_SCENES: [Scene; 2] = [Scene::Shadows, Scene::Sprite];

fn create_scene(
    scene: Scene,
    world: &mut World,
    resources: &Resources,
) -> Box<dyn TestScene> {
    match scene {
        Scene::Shadows => Box::new(ShadowsScene::new(world, resources)),
        Scene::Sprite => Box::new(SpriteScene::new(world, resources)),
    }
}

//
// All scenes implement this and new()
//
pub trait TestScene {
    fn update(
        &mut self,
        world: &mut World,
        resources: &Resources,
    );

    fn cleanup(
        &mut self,
        _world: &mut World,
        _resources: &Resources,
    ) {
    }
}

pub struct SceneManager {
    current_scene_index: usize,
    current_scene: Option<Box<dyn TestScene>>,
    next_scene: Option<usize>,
}

impl Default for SceneManager {
    fn default() -> Self {
        SceneManager {
            current_scene: None,
            current_scene_index: 0,
            next_scene: Some(0),
        }
    }
}

impl SceneManager {
    pub fn queue_load_previous_scene(&mut self) {
        if self.current_scene_index == 0 {
            self.next_scene = Some(ALL_SCENES.len() - 1)
        } else {
            self.next_scene = Some(self.current_scene_index - 1)
        }
    }

    pub fn queue_load_next_scene(&mut self) {
        self.next_scene = Some((self.current_scene_index + 1) % ALL_SCENES.len());
    }

    pub fn try_create_next_scene(
        &mut self,
        world: &mut World,
        resources: &Resources,
    ) {
        if let Some(next_scene_index) = self.next_scene.take() {
            if let Some(current_scene) = &mut self.current_scene {
                current_scene.cleanup(world, resources);
            }

            world.clear();

            let next_scene = ALL_SCENES[next_scene_index];
            log::info!("Load scene {:?}", next_scene);
            self.current_scene_index = next_scene_index;
            self.current_scene = Some(create_scene(next_scene, world, resources));
        }
    }

    pub fn update_scene(
        &mut self,
        world: &mut World,
        resources: &Resources,
    ) {
        self.current_scene
            .as_mut()
            .unwrap()
            .update(world, resources);
    }
}

fn add_light_debug_draw(
    resources: &Resources,
    world: &World,
) {
    let mut debug_draw = resources.get_mut::<DebugDraw3DResource>().unwrap();

    let mut query = <Read<DirectionalLightComponent>>::query();
    for light in query.iter(world) {
        let light_from = light.direction * -10.0;
        let light_to = glam::Vec3::zero();

        debug_draw.add_line(light_from, light_to, light.color);
    }

    let mut query = <(Read<PositionComponent>, Read<PointLightComponent>)>::query();
    for (position, light) in query.iter(world) {
        debug_draw.add_sphere(position.position, 0.25, light.color, 12);
    }

    let mut query = <(Read<PositionComponent>, Read<SpotLightComponent>)>::query();
    for (position, light) in query.iter(world) {
        let light_from = position.position;
        let light_to = position.position + light.direction;
        let light_direction = (light_to - light_from).normalize();

        debug_draw.add_cone(
            light_from,
            light_from + (light.range * light_direction),
            light.range * light.spotlight_half_angle.tan(),
            light.color,
            10,
        );
    }
}

fn add_directional_light(
    _resources: &Resources,
    world: &mut World,
    light_component: DirectionalLightComponent,
) {
    world.extend(vec![(light_component,)]);
}

fn add_spot_light(
    _resources: &Resources,
    world: &mut World,
    position: glam::Vec3,
    light_component: SpotLightComponent,
) {
    let position_component = PositionComponent { position };

    world.extend(vec![(position_component, light_component)]);
}

fn add_point_light(
    _resources: &Resources,
    world: &mut World,
    position: glam::Vec3,
    light_component: PointLightComponent,
) {
    let position_component = PositionComponent { position };

    world.extend(vec![(position_component, light_component)]);
}
