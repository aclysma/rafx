use crate::components::{
    DirectionalLightComponent, PointLightComponent, SpotLightComponent, TransformComponent,
};
use crate::features::debug3d::Debug3DResource;
use glam::Vec3;
use legion::IntoQuery;
use legion::{Read, Resources, World};
use rand::Rng;

mod shadows_scene;
use shadows_scene::ShadowsScene;

mod sprite_scene;
use sprite_scene::SpriteScene;

mod animation_scene;
use animation_scene::AnimationScene;

mod rafxmark_scene;
use rafxmark_scene::RafxmarkScene;

mod many_sprites_scene;
use many_sprites_scene::ManySpritesScene;

mod many_cubes_scene;
use many_cubes_scene::ManyCubesScene;

#[derive(Copy, Clone, Debug)]
pub enum Scene {
    Shadows,
    Sprite,
    Animation,
    Rafxmark,
    ManySprites,
    ManyCubes,
}

pub const ALL_SCENES: [Scene; 6] = [
    Scene::Shadows,
    Scene::Sprite,
    Scene::Animation,
    Scene::Rafxmark,
    Scene::ManySprites,
    Scene::ManyCubes,
];

fn random_color(rng: &mut impl Rng) -> Vec3 {
    let r = rng.gen_range(0.2..1.0);
    let g = rng.gen_range(0.2..1.0);
    let b = rng.gen_range(0.2..1.0);
    let v = Vec3::new(r, g, b);
    v.normalize()
}

fn create_scene(
    scene: Scene,
    world: &mut World,
    resources: &Resources,
) -> Box<dyn TestScene> {
    match scene {
        Scene::Shadows => Box::new(ShadowsScene::new(world, resources)),
        Scene::Sprite => Box::new(SpriteScene::new(world, resources)),
        Scene::Animation => Box::new(AnimationScene::new(world, resources)),
        Scene::Rafxmark => Box::new(RafxmarkScene::new(world, resources)),
        Scene::ManySprites => Box::new(ManySpritesScene::new(world, resources)),
        Scene::ManyCubes => Box::new(ManyCubesScene::new(world, resources)),
    }
}

//
// All scenes implement this and new()
//
pub trait TestScene {
    fn update(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
    );

    fn process_input(
        &mut self,
        _world: &mut World,
        _resources: &Resources,
        _event: &winit::event::Event<()>,
    ) {
    }

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

    pub fn process_input(
        &mut self,
        world: &mut World,
        resources: &Resources,
        event: &winit::event::Event<()>,
    ) {
        if let Some(current_scene) = &mut self.current_scene {
            current_scene.process_input(world, resources, event);
        }
    }

    pub fn has_next_scene(&self) -> bool {
        self.next_scene.is_some()
    }

    pub fn try_cleanup_current_scene(
        &mut self,
        world: &mut World,
        resources: &Resources,
    ) {
        if let Some(current_scene) = &mut self.current_scene {
            current_scene.cleanup(world, resources);
        }

        world.clear();
    }

    pub fn try_create_next_scene(
        &mut self,
        world: &mut World,
        resources: &Resources,
    ) {
        if let Some(next_scene_index) = self.next_scene.take() {
            let next_scene = ALL_SCENES[next_scene_index];
            log::info!("Load scene {:?}", next_scene);
            self.current_scene_index = next_scene_index;
            self.current_scene = Some(create_scene(next_scene, world, resources));
        }
    }

    pub fn update_scene(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
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
    let mut debug_draw = resources.get_mut::<Debug3DResource>().unwrap();

    let mut query = <Read<DirectionalLightComponent>>::query();
    for light in query.iter(world) {
        let light_from = light.direction * -10.0;
        let light_to = glam::Vec3::ZERO;

        debug_draw.add_line(light_from, light_to, light.color);
    }

    let mut query = <(Read<TransformComponent>, Read<PointLightComponent>)>::query();
    for (transform, light) in query.iter(world) {
        debug_draw.add_sphere(transform.translation, 0.1, light.color, 12);
        debug_draw.add_sphere(transform.translation, light.range, light.color, 12);
    }

    let mut query = <(Read<TransformComponent>, Read<SpotLightComponent>)>::query();
    for (transform, light) in query.iter(world) {
        let light_from = transform.translation;
        let light_to = transform.translation + light.direction;
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
    let position_component = TransformComponent {
        translation: position,
        ..Default::default()
    };

    world.extend(vec![(position_component, light_component)]);
}

fn add_point_light(
    _resources: &Resources,
    world: &mut World,
    position: glam::Vec3,
    light_component: PointLightComponent,
) {
    let position_component = TransformComponent {
        translation: position,
        ..Default::default()
    };

    world.extend(vec![(position_component, light_component)]);
}
