use glam::Vec3;
use legion::{Resources, World};
use rand::Rng;

// miscellaneous shared code between scenes
pub mod util;

//mod dungeon_scene;
//use dungeon_scene::DungeonScene;

//mod scifi_base_scene;
//use scifi_base_scene::ScifiBaseScene;

mod sprite_scene;
use sprite_scene::SpriteScene;

mod rafxmark_scene;
use rafxmark_scene::RafxmarkScene;

mod many_sprites_scene;
use many_sprites_scene::ManySpritesScene;

#[cfg(not(feature = "basic-pipeline"))]
mod bistro_scene;
#[cfg(not(feature = "basic-pipeline"))]
use bistro_scene::BistroScene;

#[cfg(not(feature = "basic-pipeline"))]
mod shadows_scene;
#[cfg(not(feature = "basic-pipeline"))]
use shadows_scene::ShadowsScene;

#[cfg(not(feature = "basic-pipeline"))]
mod autoexposure_scene;
#[cfg(not(feature = "basic-pipeline"))]
use autoexposure_scene::AutoexposureScene;

#[cfg(not(feature = "basic-pipeline"))]
mod pbr_test;
#[cfg(not(feature = "basic-pipeline"))]
use pbr_test::PbrTestScene;

#[cfg(not(feature = "basic-pipeline"))]
mod animation_scene;
#[cfg(not(feature = "basic-pipeline"))]
use animation_scene::AnimationScene;

#[cfg(not(feature = "basic-pipeline"))]
mod many_cubes_scene;
#[cfg(not(feature = "basic-pipeline"))]
use many_cubes_scene::ManyCubesScene;

#[cfg(not(feature = "basic-pipeline"))]
mod taa_test_scene;
#[cfg(not(feature = "basic-pipeline"))]
use taa_test_scene::TaaTestScene;

#[cfg(not(feature = "basic-pipeline"))]
#[derive(Copy, Clone, Debug)]
pub enum Scene {
    //ScifiBase,
    //Dungeon,
    Bistro,
    Shadows,
    PbrTest,
    Sprite,
    Animation,
    Rafxmark,
    ManySprites,
    ManyCubes,
    Autoexposure,
    TaaTestScene,
}

#[cfg(not(feature = "basic-pipeline"))]
pub const ALL_SCENES: [Scene; 9] = [
    //Scene::ScifiBase,
    //Scene::Dungeon,
    //Scene::Bistro,
    Scene::Shadows,
    Scene::PbrTest,
    Scene::Sprite,
    Scene::Animation,
    Scene::Rafxmark,
    Scene::ManySprites,
    Scene::Autoexposure,
    Scene::ManyCubes,
    Scene::TaaTestScene,
];

#[cfg(feature = "basic-pipeline")]
#[derive(Copy, Clone, Debug)]
pub enum Scene {
    Sprite,
    Rafxmark,
    ManySprites,
}

#[cfg(feature = "basic-pipeline")]
pub const ALL_SCENES: [Scene; 3] = [Scene::Sprite, Scene::Rafxmark, Scene::ManySprites];

fn random_color(rng: &mut impl Rng) -> Vec3 {
    let r = rng.gen_range(0.2..1.0);
    let g = rng.gen_range(0.2..1.0);
    let b = rng.gen_range(0.2..1.0);
    let v = Vec3::new(r, g, b);
    v.normalize()
}

#[cfg(not(feature = "basic-pipeline"))]
fn create_scene(
    scene: Scene,
    world: &mut World,
    resources: &Resources,
) -> Box<dyn TestScene> {
    match scene {
        //Scene::Dungeon => Box::new(DungeonScene::new(world, resources)),
        //Scene::ScifiBase => Box::new(ScifiBaseScene::new(world, resources)),
        Scene::Bistro => Box::new(BistroScene::new(world, resources)),
        Scene::Shadows => Box::new(ShadowsScene::new(world, resources)),
        Scene::PbrTest => Box::new(PbrTestScene::new(world, resources)),
        Scene::Sprite => Box::new(SpriteScene::new(world, resources)),
        Scene::Animation => Box::new(AnimationScene::new(world, resources)),
        Scene::Rafxmark => Box::new(RafxmarkScene::new(world, resources)),
        Scene::ManySprites => Box::new(ManySpritesScene::new(world, resources)),
        Scene::ManyCubes => Box::new(ManyCubesScene::new(world, resources)),
        Scene::Autoexposure => Box::new(AutoexposureScene::new(world, resources)),
        Scene::TaaTestScene => Box::new(TaaTestScene::new(world, resources)),
    }
}

#[cfg(feature = "basic-pipeline")]
fn create_scene(
    scene: Scene,
    world: &mut World,
    resources: &Resources,
) -> Box<dyn TestScene> {
    match scene {
        Scene::Sprite => Box::new(SpriteScene::new(world, resources)),
        Scene::Rafxmark => Box::new(RafxmarkScene::new(world, resources)),
        Scene::ManySprites => Box::new(ManySpritesScene::new(world, resources)),
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
