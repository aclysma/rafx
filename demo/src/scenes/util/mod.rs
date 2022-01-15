use crate::RenderOptions;
use legion::IntoQuery;
use legion::{Read, Resources, World};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::ImageAsset;
use rafx::render_features::{
    RenderFeatureFlagMaskBuilder, RenderFeatureMaskBuilder, RenderPhaseMaskBuilder,
};
use rafx_plugins::components::{
    DirectionalLightComponent, PointLightComponent, SpotLightComponent, TransformComponent,
};
use rafx_plugins::features::debug3d::{Debug3DRenderFeature, Debug3DResource};
use rafx_plugins::features::skybox::{SkyboxRenderFeature, SkyboxResource};
use rafx_plugins::features::sprite::SpriteRenderFeature;
use rafx_plugins::features::text::TextRenderFeature;
use rafx_plugins::features::tile_layer::TileLayerRenderFeature;
use rafx_plugins::phases::{
    DebugPipRenderPhase, DepthPrepassRenderPhase, OpaqueRenderPhase, TransparentRenderPhase,
    UiRenderPhase, WireframeRenderPhase,
};

#[cfg(feature = "basic-pipeline")]
use rafx_plugins::features::mesh_basic::{
    MeshBasicNoShadowsRenderFeatureFlag as MeshNoShadowsRenderFeatureFlag,
    MeshBasicRenderFeature as MeshRenderFeature, MeshBasicRenderOptions as MeshRenderOptions,
    MeshBasicUnlitRenderFeatureFlag as MeshUnlitRenderFeatureFlag,
    MeshBasicUntexturedRenderFeatureFlag as MeshUntexturedRenderFeatureFlag,
    MeshBasicWireframeRenderFeatureFlag as MeshWireframeRenderFeatureFlag,
};

#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::features::mesh_adv::{
    MeshAdvNoShadowsRenderFeatureFlag as MeshNoShadowsRenderFeatureFlag,
    MeshAdvRenderFeature as MeshRenderFeature, MeshAdvRenderOptions as MeshRenderOptions,
    MeshAdvUnlitRenderFeatureFlag as MeshUnlitRenderFeatureFlag,
    MeshAdvUntexturedRenderFeatureFlag as MeshUntexturedRenderFeatureFlag,
    MeshAdvWireframeRenderFeatureFlag as MeshWireframeRenderFeatureFlag,
};

mod fly_camera;
pub use fly_camera::*;

mod path_camera;
pub use path_camera::*;

mod demo_camera;
pub use demo_camera::*;

use rafx::visibility::VisibilityResource;

mod spawnable_mesh;
pub use spawnable_mesh::*;

mod spawnable_prefab;
pub use spawnable_prefab::*;

pub(super) fn set_ambient_light(
    resources: &Resources,
    ambient_light: glam::Vec3,
) {
    let mut mesh_render_options = resources.get_mut::<MeshRenderOptions>().unwrap();
    mesh_render_options.ambient_light = ambient_light;
}

pub fn add_light_debug_draw(
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
        debug_draw.add_sphere(transform.translation, light.range(), light.color, 12);
    }

    let mut query = <(Read<TransformComponent>, Read<SpotLightComponent>)>::query();
    for (transform, light) in query.iter(world) {
        let light_from = transform.translation;
        let light_to = transform.translation + light.direction;
        let light_direction = (light_to - light_from).normalize();

        debug_draw.add_cone(
            light_from,
            light_from + (light.range() * light_direction),
            light.range() * light.spotlight_half_angle.tan(),
            light.color,
            10,
        );
    }
}

pub(super) fn add_directional_light(
    resources: &Resources,
    world: &mut World,
    direction: glam::Vec3,
    color: glam::Vec4,
    intensity: f32,
    cast_shadows: bool,
) {
    let view_frustum = if cast_shadows {
        let mut visibility_resource = resources.get_mut::<VisibilityResource>().unwrap();
        Some(visibility_resource.register_view_frustum())
    } else {
        None
    };

    let light_component = DirectionalLightComponent {
        direction,
        color,
        intensity,
        shadow_view_frustum: view_frustum,
    };

    world.extend(vec![(light_component,)]);
}

pub(super) fn add_spot_light(
    resources: &Resources,
    world: &mut World,
    position: glam::Vec3,
    direction: glam::Vec3,
    spotlight_half_angle: f32,
    color: glam::Vec4,
    intensity: f32,
    cast_shadows: bool,
) {
    let view_frustum = if cast_shadows {
        let mut visibility_resource = resources.get_mut::<VisibilityResource>().unwrap();
        Some(visibility_resource.register_view_frustum())
    } else {
        None
    };

    let position_component = TransformComponent {
        translation: position,
        ..Default::default()
    };

    const MAX_HALF_ANGLE_DEG: f32 = 80.0;
    const MAX_HALF_HANGLE_RAD: f32 = MAX_HALF_ANGLE_DEG * (std::f32::consts::PI / 180.0);

    let light_component = SpotLightComponent {
        direction,
        spotlight_half_angle: spotlight_half_angle.min(MAX_HALF_HANGLE_RAD),
        range: None,
        color,
        intensity,
        shadow_view_frustum: view_frustum,
    };

    world.extend(vec![(position_component, light_component)]);
}

pub(super) fn add_point_light(
    resources: &Resources,
    world: &mut World,
    position: glam::Vec3,
    color: glam::Vec4,
    intensity: f32,
    cast_shadows: bool,
) {
    let view_frustums = if cast_shadows {
        let mut visibility_resource = resources.get_mut::<VisibilityResource>().unwrap();
        Some([
            visibility_resource.register_view_frustum(),
            visibility_resource.register_view_frustum(),
            visibility_resource.register_view_frustum(),
            visibility_resource.register_view_frustum(),
            visibility_resource.register_view_frustum(),
            visibility_resource.register_view_frustum(),
        ])
    } else {
        None
    };

    let position_component = TransformComponent {
        translation: position,
        ..Default::default()
    };

    let light_component = PointLightComponent {
        color,
        intensity,
        range: None,
        shadow_view_frustums: view_frustums,
    };

    world.extend(vec![(position_component, light_component)]);
}

pub fn default_main_view_masks(
    render_options: &RenderOptions
) -> (
    RenderPhaseMaskBuilder,
    RenderFeatureMaskBuilder,
    RenderFeatureFlagMaskBuilder,
) {
    let phase_mask_builder = RenderPhaseMaskBuilder::default()
        .add_render_phase::<DepthPrepassRenderPhase>()
        .add_render_phase::<OpaqueRenderPhase>()
        .add_render_phase::<TransparentRenderPhase>()
        .add_render_phase::<WireframeRenderPhase>()
        .add_render_phase::<DebugPipRenderPhase>()
        .add_render_phase::<UiRenderPhase>();

    //use rafx_plugins::features::debug_pip::DebugPipRenderFeature;
    let mut feature_mask_builder = RenderFeatureMaskBuilder::default()
        .add_render_feature::<MeshRenderFeature>()
        .add_render_feature::<SpriteRenderFeature>()
        //.add_render_feature::<DebugPipRenderFeature>()
        .add_render_feature::<TileLayerRenderFeature>();

    #[cfg(feature = "egui")]
    {
        feature_mask_builder = feature_mask_builder
            .add_render_feature::<rafx_plugins::features::egui::EguiRenderFeature>();
    }

    if render_options.show_text {
        feature_mask_builder = feature_mask_builder.add_render_feature::<TextRenderFeature>();
    }

    if render_options.show_debug3d {
        feature_mask_builder = feature_mask_builder.add_render_feature::<Debug3DRenderFeature>();
    }

    if render_options.show_skybox {
        feature_mask_builder = feature_mask_builder.add_render_feature::<SkyboxRenderFeature>();
    }

    let mut feature_flag_mask_builder = RenderFeatureFlagMaskBuilder::default();

    if render_options.show_wireframes {
        feature_flag_mask_builder =
            feature_flag_mask_builder.add_render_feature_flag::<MeshWireframeRenderFeatureFlag>();
    }

    if !render_options.enable_lighting {
        feature_flag_mask_builder =
            feature_flag_mask_builder.add_render_feature_flag::<MeshUnlitRenderFeatureFlag>();
    }

    if !render_options.enable_textures {
        feature_flag_mask_builder =
            feature_flag_mask_builder.add_render_feature_flag::<MeshUntexturedRenderFeatureFlag>();
    }

    if !render_options.show_shadows {
        feature_flag_mask_builder =
            feature_flag_mask_builder.add_render_feature_flag::<MeshNoShadowsRenderFeatureFlag>();
    }

    (
        phase_mask_builder,
        feature_mask_builder,
        feature_flag_mask_builder,
    )
}

//"textures/skybox.basis"
pub fn setup_skybox<T: Into<String>>(
    resources: &Resources,
    path: T,
) {
    let asset_resource = resources.get::<AssetResource>().unwrap();
    let skybox_texture = asset_resource.load_asset_path::<ImageAsset, _>(path);

    *resources
        .get_mut::<SkyboxResource>()
        .unwrap()
        .skybox_texture_mut() = Some(skybox_texture);
}
