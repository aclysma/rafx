// NOTE(dvd): Inspired by Bevy `many_sprites` example (MIT licensed) https://github.com/bevyengine/bevy/blob/621cba4864fd5d2c0962151b126769eff45797fd/examples/2d/many_sprites.rs

use crate::phases::{
    DepthPrepassRenderPhase, OpaqueRenderPhase, TransparentRenderPhase, UiRenderPhase,
};
use crate::time::TimeState;
use crate::RenderOptions;
use glam::{Quat, Vec2, Vec3};
use legion;
use legion::{IntoQuery, Read, Resources, Schedule, SystemBuilder, World, Write};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::ImageAsset;
use rafx::distill::loader::handle::Handle;
use rafx::rafx_visibility::{DepthRange, OrthographicParameters, Projection};
use rafx::render_features::{
    RenderFeatureFlagMask, RenderFeatureMaskBuilder, RenderPhaseMaskBuilder, RenderViewDepthRange,
};
use rafx::renderer::{RenderViewMeta, ViewportsResource};
use rafx::visibility::{CullModel, ObjectId, ViewFrustumArc, VisibilityRegion};
use rafx_plugins::assets::font::FontAsset;
use rafx_plugins::components::SpriteComponent;
use rafx_plugins::components::{TransformComponent, VisibilityComponent};
use rafx_plugins::features::skybox::SkyboxRenderFeature;
use rafx_plugins::features::sprite::{
    SpriteRenderFeature, SpriteRenderObject, SpriteRenderObjectSet,
};
use rafx_plugins::features::text::{TextRenderFeature, TextResource};
use rand::Rng;

const CAMERA_SPEED: f32 = 1000.0;

#[derive(Clone, Copy)]
struct CameraComponent {
    position: TransformComponent,
    up: Vec3,
}

#[derive(Clone)]
struct TextComponent {
    text: String,
    font: Handle<FontAsset>,
}

pub(super) struct ManySpritesScene {
    schedule: Schedule,
    main_view_frustum: ViewFrustumArc,
}

impl ManySpritesScene {
    pub(super) fn new(
        world: &mut World,
        resources: &Resources,
    ) -> Self {
        let mut render_options = resources.get_mut::<RenderOptions>().unwrap();
        *render_options = RenderOptions::default_2d();

        let visibility_region = resources.get::<VisibilityRegion>().unwrap();

        let sprite_image = {
            let asset_resource = resources.get::<AssetResource>().unwrap();
            asset_resource.load_asset_path::<ImageAsset, _>("textures/texture-tiny-rust.jpeg")
        };

        let font = {
            let asset_resource = resources.get::<AssetResource>().unwrap();
            asset_resource.load_asset_path::<FontAsset, _>("fonts/mplus-1p-regular.ttf")
        };

        let mut rng = rand::thread_rng();

        let tile_size = Vec2::splat(64.0);
        let map_size = Vec2::splat(320.0);

        let half_x = (map_size.x / 2.0) as i32;
        let half_y = (map_size.y / 2.0) as i32;

        let main_view_frustum = visibility_region.register_view_frustum();
        let mut main_view_frustum_copy = main_view_frustum.clone();

        let update_camera_system = SystemBuilder::new("update_camera")
            .read_resource::<TimeState>()
            .write_resource::<ViewportsResource>()
            .with_query(<Write<CameraComponent>>::query())
            .build(move |_, world, (time_state, viewports_resource), queries| {
                profiling::scope!("update_camera_system");
                for camera in queries.iter_mut(world) {
                    update_main_view_2d(
                        camera,
                        &mut main_view_frustum_copy,
                        time_state,
                        viewports_resource,
                    );
                }
            });

        let update_transforms_system = SystemBuilder::new("update_transforms")
            .read_resource::<TimeState>()
            .with_query(<Write<TransformComponent>>::query())
            .build(move |_, world, time, queries| {
                profiling::scope!("update_transforms_system");
                for transform in queries.iter_mut(world) {
                    transform.rotation *=
                        Quat::from_rotation_z(time.previous_update_dt() * rand::random::<f32>());
                }
            });

        let print_text_system = SystemBuilder::new("print_text")
            .write_resource::<TextResource>()
            .with_query(<Read<TextComponent>>::query())
            .build(move |_, world, text_resource, queries| {
                profiling::scope!("print_text_system");
                for text in queries.iter_mut(world) {
                    text_resource.add_text(
                        text.text.clone(),
                        glam::Vec3::new(25.0, 25.0, 0.0),
                        &text.font,
                        40.0,
                        glam::Vec4::new(1.0, 0.0, 0.0, 1.0),
                    );
                }
            });

        let schedule = Schedule::builder()
            .add_system(update_camera_system)
            .add_system(update_transforms_system)
            .add_system(print_text_system)
            .build();

        let mut sprite_count = 0 as usize;

        for y in -half_y..half_y {
            for x in -half_x..half_x {
                let position = Vec2::new(x as f32, y as f32);
                let translation = (position * tile_size).extend(rng.gen::<f32>());
                let scale = Vec3::new(
                    rng.gen::<f32>() * 2.0,
                    rng.gen::<f32>() * 2.0,
                    rng.gen::<f32>() * 2.0,
                );

                let tint = super::random_color(&mut rng);
                let alpha = f32::max(0.2, rng.gen::<f32>());

                let mut sprite_render_objects =
                    resources.get_mut::<SpriteRenderObjectSet>().unwrap();

                let sprite_render_object =
                    sprite_render_objects.register_render_object(SpriteRenderObject {
                        tint,
                        alpha,
                        image: sprite_image.clone(),
                    });

                let transform_component = TransformComponent {
                    translation,
                    scale,
                    rotation: Quat::from_rotation_ypr(
                        rng.gen::<f32>(),
                        rng.gen::<f32>(),
                        rng.gen::<f32>(),
                    ),
                };

                let sprite_component = SpriteComponent {
                    render_object_handle: sprite_render_object.clone(),
                };

                let entity = world.push((transform_component.clone(), sprite_component));
                let mut entry = world.entry(entity).unwrap();
                entry.add_component(VisibilityComponent {
                    visibility_object_handle: {
                        let handle = visibility_region.register_dynamic_object(
                            ObjectId::from(entity),
                            CullModel::quad(64., 64.),
                        );
                        handle.set_transform(
                            transform_component.translation,
                            transform_component.rotation,
                            transform_component.scale,
                        );
                        handle.add_render_object(&sprite_render_object);
                        handle
                    },
                });

                sprite_count += 1;
            }
        }

        const CAMERA_Z: f32 = 1000.0;

        world.push((CameraComponent {
            position: TransformComponent {
                translation: Vec3::new(0., 0., CAMERA_Z),
                rotation: Quat::from_rotation_x(0.),
                scale: Vec3::new(1., 1., 1.),
            },
            up: Vec3::new(0., 1., 0.),
        },));

        world.push((TextComponent {
            text: format!("Sprite Count: {}", sprite_count),
            font,
        },));

        ManySpritesScene {
            schedule,
            main_view_frustum: main_view_frustum,
        }
    }
}

impl super::TestScene for ManySpritesScene {
    fn update(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
    ) {
        self.schedule.execute(world, resources);
    }
}

#[profiling::function]
fn update_main_view_2d(
    camera: &mut CameraComponent,
    main_view_frustum: &mut ViewFrustumArc,
    time: &TimeState,
    viewports_resource: &mut ViewportsResource,
) {
    let main_camera_phase_mask = RenderPhaseMaskBuilder::default()
        .add_render_phase::<DepthPrepassRenderPhase>()
        .add_render_phase::<OpaqueRenderPhase>()
        .add_render_phase::<TransparentRenderPhase>()
        .add_render_phase::<UiRenderPhase>()
        .build();

    let mut main_camera_feature_mask = RenderFeatureMaskBuilder::default();
    main_camera_feature_mask = main_camera_feature_mask
        .add_render_feature::<SkyboxRenderFeature>()
        .add_render_feature::<SpriteRenderFeature>()
        .add_render_feature::<TextRenderFeature>();

    #[cfg(feature = "egui")]
    {
        main_camera_feature_mask = main_camera_feature_mask
            .add_render_feature::<rafx_plugins::features::egui::EguiRenderFeature>(
        );
    }

    let main_camera_feature_mask = main_camera_feature_mask.build();

    // Round to a whole number

    camera
        .position
        .rotate(Quat::from_rotation_z(time.previous_update_dt() * 0.75));
    camera.position = camera.position.mul_transform(TransformComponent {
        translation: Vec3::new(1., 0., 0.) * CAMERA_SPEED * time.previous_update_dt(),
        rotation: Quat::from_rotation_x(0.),
        scale: Vec3::new(1., 1., 1.),
    });

    let mut eye = camera.position.translation;

    let mut transform = TransformComponent {
        translation: eye,
        rotation: Quat::from_rotation_x(0.),
        scale: Vec3::new(1., 1., 1.),
    };

    transform.rotate(Quat::from_rotation_z(time.previous_update_dt() / 2.0));

    camera.up = transform.mul_vec3(camera.up);

    let half_width = viewports_resource.main_window_size.width as f32 / 2.0;
    let half_height = viewports_resource.main_window_size.height as f32 / 2.0;

    //
    // This logic ensures pixel-perfect rendering when we have odd-sized width/height dimensions.
    // We also need to round x/y to whole numbers to render pixel-perfect
    //
    if viewports_resource.main_window_size.width % 2 != 0 {
        eye.x = eye.x.round() + 0.5;
    } else {
        eye.x = eye.x.round();
    }

    if viewports_resource.main_window_size.height % 2 != 0 {
        eye.y = eye.y.round() + 0.5;
    } else {
        eye.y = eye.y.round();
    }

    let look_at = Vec3::new(0., 0., 0.);

    let view = glam::Mat4::look_at_rh(eye, look_at, camera.up);

    let near = 0.01;
    let far = 10000.0;

    let projection = Projection::Orthographic(OrthographicParameters::new(
        -half_width,
        half_width,
        -half_height,
        half_height,
        near,
        far,
        DepthRange::InfiniteReverse,
    ));

    main_view_frustum
        .set_projection(&projection)
        .set_transform(eye, look_at, camera.up);

    viewports_resource.main_view_meta = Some(RenderViewMeta {
        view_frustum: main_view_frustum.clone(),
        eye_position: eye,
        view,
        proj: projection.as_rh_mat4(),
        depth_range: RenderViewDepthRange::from_projection(&projection),
        render_phase_mask: main_camera_phase_mask,
        render_feature_mask: main_camera_feature_mask,
        render_feature_flag_mask: RenderFeatureFlagMask::empty(),
        debug_name: "main".to_string(),
    });
}
