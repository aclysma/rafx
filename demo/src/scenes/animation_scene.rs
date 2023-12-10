use crate::time::TimeState;
use crate::RenderOptions;
use glam::f32::Vec3;
use glam::Quat;
use legion::{Resources, World};
use rafx::assets::AssetManager;
use rafx::assets::AssetResource;
use rafx::rafx_visibility::{DepthRange, PerspectiveParameters, Projection};
use rafx::render_features::RenderViewDepthRange;
use rafx::renderer::{RenderViewMeta, Renderer, ViewportsResource};
use rafx::visibility::{ViewFrustumArc, VisibilityResource};
use rafx_plugins::assets::anim::{AnimAsset, AnimClip, Skeleton};
use rafx_plugins::features::debug3d::Debug3DResource;
use std::sync::Arc;

#[derive(Debug)]
struct PosedBone {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
}

fn pose(
    skeleton: &Skeleton,
    anim_clip: &AnimClip,
    frame: u32,
) -> Vec<PosedBone> {
    let mut posed_bones = Vec::<PosedBone>::with_capacity(skeleton.bones.len());
    for (bone, channel_group) in skeleton.bones.iter().zip(&anim_clip.bone_channel_groups) {
        // The base pose position of this bone, offset from the parent bone (or base orientation if there is no parent)
        let rest_position = bone.position_rel;
        let rest_rotation = bone.rotation_rel;
        let mut anim_position = Vec3::ZERO;
        let mut anim_rotation = Quat::IDENTITY;

        // Apply animated position change. This position change is in local space of the bone so we can simply add it
        if let Some(position_channel_group) = &channel_group.position {
            let frame_offset = frame.clamp(
                position_channel_group.min_frame,
                position_channel_group.max_frame,
            );
            anim_position = position_channel_group.values
                [(frame_offset - position_channel_group.min_frame) as usize]
                .into();
        }

        // Apply animated rotation change. This rotation change is also in local space of the bone
        if let Some(rotation_channel_group) = &channel_group.rotation {
            let frame_offset = frame.clamp(
                rotation_channel_group.min_frame,
                rotation_channel_group.max_frame,
            );
            anim_rotation = rotation_channel_group.values
                [(frame_offset - rotation_channel_group.min_frame) as usize]
                .into();
            //TODO: Do this on import and maybe during blender export, doing it here is a hack
            anim_rotation = anim_rotation.normalize();
        }

        // We don't use our own animated rotation here because our rotation affects the end of the bone
        // (i.e. where the next bone begins) but does not affect the joint itself
        //NOTE: I changed this so anim_position is transformed by rest_rotation
        let mut position = rest_position + rest_rotation.mul_vec3(anim_position);
        let mut rotation = rest_rotation.mul_quat(anim_rotation);
        //println!("REST {:?} ANIM {:?} ROT {:?}", rest_rotation, anim_rotation, rotation);

        if bone.parent != -1 {
            let parent_pose = &posed_bones[bone.parent as usize];
            //println!("add {:?}",parent_pose.rotation.mul_vec3(position));
            position = parent_pose.position + parent_pose.rotation.mul_vec3(position);
            rotation = parent_pose.rotation.mul_quat(rotation);
        }

        let normalization_error = 1.0 - rotation.length();
        if !rotation.is_normalized() {
            println!(
                "Bone {} rotation not normalized, normalization error: {}",
                bone.name, normalization_error
            );
        }

        //TODO: Maybe worth doing if we compute a full skeleton animation once and reference it
        rotation = rotation.normalize();

        //println!("Bone {:?} anim_position {:?} position {:?}", bone.name, anim_position, position);

        posed_bones.push(PosedBone { position, rotation });
    }

    posed_bones
}

pub(super) struct AnimationScene {
    main_view_frustum: ViewFrustumArc,
    skeleton: Arc<Skeleton>,
    anim_clip: Arc<AnimClip>,
}

impl AnimationScene {
    pub(super) fn new(
        _world: &mut World,
        resources: &Resources,
    ) -> Self {
        let mut visibility_resource = resources.get_mut::<VisibilityResource>().unwrap();
        let main_view_frustum = visibility_resource.register_view_frustum();

        let mut asset_manager = resources.get_mut::<AssetManager>().unwrap();
        let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();
        let renderer = resources.get::<Renderer>().unwrap();
        let anim_asset = asset_resource
            .load_asset_symbol_name::<AnimAsset>("db:/assets/demo/armature/Armature.blender_anim");

        renderer
            .wait_for_asset_to_load(&mut asset_manager, &anim_asset, &mut asset_resource, "")
            .unwrap();

        let anim_asset_data = asset_manager.committed_asset(&anim_asset).unwrap();

        let mut render_options = resources.get_mut::<RenderOptions>().unwrap();
        *render_options = RenderOptions::default_3d();
        render_options.show_skybox = false;

        AnimationScene {
            main_view_frustum,
            skeleton: anim_asset_data.skeleton().clone(),
            anim_clip: anim_asset_data.clip(0).clone(),
        }
    }
}

impl super::TestScene for AnimationScene {
    fn update(
        &mut self,
        _world: &mut World,
        resources: &mut Resources,
    ) {
        {
            let time_state = resources.get::<TimeState>().unwrap();
            let mut viewports_resource = resources.get_mut::<ViewportsResource>().unwrap();
            let render_options = resources.get::<RenderOptions>().unwrap();

            update_main_view_3d(
                &*time_state,
                &*render_options,
                &mut self.main_view_frustum,
                &mut *viewports_resource,
            );
        }

        let time_state = resources.get::<TimeState>().unwrap();

        let frame = (time_state.total_time().as_secs_f32() * 30.0) as u32 % 100;

        let mut debug_draw = resources.get_mut::<Debug3DResource>().unwrap();

        let skeleton = &*self.skeleton;
        let pose = pose(skeleton, &*self.anim_clip, frame);
        let max_depth = self
            .skeleton
            .bones
            .iter()
            .map(|x| x.chain_depth)
            .max()
            .unwrap_or(1)
            .max(1);

        #[derive(Copy, Clone, Debug, PartialEq)]
        enum JointDrawStyle {
            ColorByDepth,
            ColorConstant,
            Disabled,
        }

        enum JointOrientationDrawStyle {
            Enabled,
            Disabled,
        }

        #[derive(Copy, Clone, Debug, PartialEq)]
        enum BoneDrawStyle {
            ColorByDepth,
            ColorConstant,
            Disabled,
        }

        let joint_draw_style = JointDrawStyle::ColorConstant;
        let bone_draw_style = BoneDrawStyle::ColorByDepth;
        //println!("skeleton data f={} p={} r={}", frame, pose[0].position, pose[0].rotation);
        for (b, p) in skeleton.bones.iter().zip(&pose) {
            //println!("p: {:?} b: {:?}", p, b);
            if joint_draw_style != JointDrawStyle::Disabled {
                let depth_percent = b.chain_depth as f32 / max_depth.max(1) as f32;
                let color = if joint_draw_style == JointDrawStyle::ColorByDepth {
                    glam::Vec3::X.lerp(glam::Vec3::Z, depth_percent).extend(1.0)
                } else {
                    glam::Vec4::new(1.0, 1.0, 0.0, 1.0)
                };

                debug_draw.add_sphere(p.position, 0.02, color, 8);
            }

            if bone_draw_style != BoneDrawStyle::Disabled && b.parent != -1 {
                let depth_percent = b.chain_depth as f32 / max_depth.max(1) as f32;
                //let parent_bone = &skeleton.bones[b.parent as usize];
                let parent_pose = &pose[b.parent as usize];
                let color = if bone_draw_style == BoneDrawStyle::ColorByDepth {
                    glam::Vec3::X.lerp(glam::Vec3::Z, depth_percent).extend(1.0)
                } else {
                    glam::Vec4::new(1.0, 1.0, 0.0, 1.0)
                };

                debug_draw.add_line(parent_pose.position, p.position, color);
            }

            debug_draw.add_line(
                p.position,
                p.position + p.rotation.mul_vec3(Vec3::X * 0.1),
                glam::Vec3::X.extend(1.0),
            );
            debug_draw.add_line(
                p.position,
                p.position + p.rotation.mul_vec3(Vec3::Y * 0.1),
                glam::Vec3::Y.extend(1.0),
            );
            debug_draw.add_line(
                p.position,
                p.position + p.rotation.mul_vec3(Vec3::Z * 0.1),
                glam::Vec3::Z.extend(1.0),
            );
        }
        //
        // debug_draw.add_sphere(glam::Vec3::new(0.0, 0.0, 0.0), 0.25, glam::Vec3::X.extend(1.0), 8);
        // debug_draw.add_sphere(glam::Vec3::new(0.0, 0.0, 1.0), 0.25, glam::Vec3::Y.extend(1.0), 8);
        // debug_draw.add_sphere(glam::Vec3::new(0.0, 0.0, 2.0), 0.25, glam::Vec3::Z.extend(1.0), 8);

        debug_draw.add_axis_aligned_grid(1.0);
    }

    fn cleanup(
        &mut self,
        _world: &mut World,
        _resources: &Resources,
    ) {
    }
}

#[profiling::function]
fn update_main_view_3d(
    time_state: &TimeState,
    render_options: &RenderOptions,
    main_view_frustum: &mut ViewFrustumArc,
    viewports_resource: &mut ViewportsResource,
) {
    let (phase_mask_builder, feature_mask_builder, feature_flag_mask_builder) =
        super::util::default_main_view_masks(render_options);

    const CAMERA_XY_DISTANCE: f32 = 8.0;
    const CAMERA_Z: f32 = 3.0;
    const CAMERA_ROTATE_SPEED: f32 = -0.10;
    const CAMERA_LOOP_OFFSET: f32 = -0.3;
    let loop_time = time_state.total_time().as_secs_f32();
    let eye = glam::Vec3::new(
        CAMERA_XY_DISTANCE * f32::cos(CAMERA_ROTATE_SPEED * loop_time + CAMERA_LOOP_OFFSET),
        CAMERA_XY_DISTANCE * f32::sin(CAMERA_ROTATE_SPEED * loop_time + CAMERA_LOOP_OFFSET),
        CAMERA_Z,
    );

    let aspect_ratio = viewports_resource.main_window_size.width as f32
        / viewports_resource.main_window_size.height as f32;

    let look_at = glam::Vec3::Z;
    let up = glam::Vec3::new(0.0, 0.0, 1.0);
    let view = glam::Mat4::look_at_rh(eye, look_at, up);

    let fov_y_radians = std::f32::consts::FRAC_PI_4;
    let near_plane = 0.01;

    let projection = Projection::Perspective(PerspectiveParameters::new(
        fov_y_radians,
        aspect_ratio,
        near_plane,
        10000.,
        DepthRange::InfiniteReverse,
    ));

    main_view_frustum
        .set_projection(&projection)
        .set_transform(eye, look_at, up);

    viewports_resource.main_view_meta = Some(RenderViewMeta {
        view_frustum: main_view_frustum.clone(),
        eye_position: eye,
        view,
        proj: projection.as_rh_mat4(),
        depth_range: RenderViewDepthRange::from_projection(&projection),
        render_phase_mask: phase_mask_builder.build(),
        render_feature_mask: feature_mask_builder.build(),
        render_feature_flag_mask: feature_flag_mask_builder.build(),
        debug_name: "main".to_string(),
    });
}
