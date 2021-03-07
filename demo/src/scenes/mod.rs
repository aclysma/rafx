use crate::components::{
    DirectionalLightComponent, PointLightComponent, PositionComponent, SpotLightComponent,
};
use crate::features::debug3d::DebugDraw3DResource;
use legion::IntoQuery;
use legion::{Read, Resources, World};

mod shadows_scene;
use shadows_scene::ShadowsScene;

mod sprite_scene;
use crate::phases::{OpaqueRenderPhase, TransparentRenderPhase, UiRenderPhase};
use crate::time::TimeState;
use rafx::nodes::{RenderPhaseMaskBuilder, RenderViewDepthRange};
use rafx::renderer::{RenderViewMeta, ViewportsResource};
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

#[profiling::function]
fn update_main_view(
    time_state: &TimeState,
    viewports_resource: &mut ViewportsResource,
) {
    let main_camera_render_phase_mask = RenderPhaseMaskBuilder::default()
        .add_render_phase::<OpaqueRenderPhase>()
        .add_render_phase::<TransparentRenderPhase>()
        .add_render_phase::<UiRenderPhase>()
        .build();

    const CAMERA_XY_DISTANCE: f32 = 12.0;
    const CAMERA_Z: f32 = 6.0;
    const CAMERA_ROTATE_SPEED: f32 = -0.10;
    const CAMERA_LOOP_OFFSET: f32 = -0.3;
    let loop_time = time_state.total_time().as_secs_f32();
    let eye = glam::Vec3::new(
        CAMERA_XY_DISTANCE * f32::cos(CAMERA_ROTATE_SPEED * loop_time + CAMERA_LOOP_OFFSET),
        CAMERA_XY_DISTANCE * f32::sin(CAMERA_ROTATE_SPEED * loop_time + CAMERA_LOOP_OFFSET),
        CAMERA_Z,
    );

    let aspect_ratio = viewports_resource.main_window_size.width as f32
        / viewports_resource.main_window_size.height.max(1) as f32;

    let view = glam::Mat4::look_at_rh(eye, glam::Vec3::zero(), glam::Vec3::new(0.0, 0.0, 1.0));

    let near_plane = 0.01;
    let proj = glam::Mat4::perspective_infinite_reverse_rh(
        std::f32::consts::FRAC_PI_4,
        aspect_ratio,
        near_plane,
    );

    viewports_resource.main_view_meta = Some(RenderViewMeta {
        eye_position: eye,
        view,
        proj,
        depth_range: RenderViewDepthRange::new_infinite_reverse(near_plane),
        render_phase_mask: main_camera_render_phase_mask,
        debug_name: "main".to_string(),
    });

    // render_view_set.create_view(
    //     eye,
    //     view,
    //     proj,
    //     (window_width, window_height),
    //     RenderViewDepthRange::new_infinite_reverse(near_plane),
    //     main_camera_render_phase_mask,
    //     "main".to_string(),
    // )
}
