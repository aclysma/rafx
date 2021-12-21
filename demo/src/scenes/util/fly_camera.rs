use crate::input::{InputState, KeyboardKey};
use crate::time::TimeState;

//
// Camera by default points along +X axis, +Z up
//
#[derive(Default)]
pub struct FlyCamera {
    pub position: glam::Vec3,
    pub look_dir: glam::Vec3,
    pub right_dir: glam::Vec3,
    pub up_dir: glam::Vec3,
    pub pitch: f32,
    pub yaw: f32,
    pub lock_view: bool,
}

impl FlyCamera {
    pub fn update(
        &mut self,
        input_state: &InputState,
        time_state: &TimeState,
    ) {
        // Allow locking camera position/rotation
        if input_state.is_key_just_down(KeyboardKey::F) {
            self.lock_view = !self.lock_view;
        }

        const NORMAL_MOVE_SPEED: f32 = 10.0;
        const FAST_MOVE_SPEED: f32 = 30.0;
        const LOOK_SPEED: f32 = 0.1;
        const TWO_PI: f32 = 2.0 * std::f32::consts::PI;

        // Use mouse motion to rotate the camera
        if !self.lock_view {
            let yaw_dt = input_state.mouse_motion().x as f32 * LOOK_SPEED * -1.0;
            let pitch_dt = input_state.mouse_motion().y as f32 * LOOK_SPEED * -1.0;

            self.yaw += yaw_dt * time_state.previous_update_dt();
            while self.yaw > std::f32::consts::PI {
                self.yaw -= TWO_PI;
            }

            while self.yaw < -std::f32::consts::PI {
                self.yaw += TWO_PI
            }

            self.pitch += pitch_dt * time_state.previous_update_dt();
            self.pitch = self.pitch.clamp(
                -std::f32::consts::FRAC_PI_2 + 0.01,
                std::f32::consts::FRAC_PI_2 - 0.01,
            );
            self.pitch += pitch_dt * time_state.previous_update_dt();
            self.pitch = self.pitch.clamp(
                -std::f32::consts::FRAC_PI_2 + 0.01,
                std::f32::consts::FRAC_PI_2 - 0.01,
            );
        }

        // Recalculate frenet frame, do this even if the camera is locked so that if the pitch/yaw
        // is set manually, the directions refresh
        // Z-Up
        let z = self.pitch.sin();
        let z_inv = 1.0 - z.abs();
        let x = self.yaw.cos() * z_inv;
        let y = self.yaw.sin() * z_inv;
        let look_dir = glam::Vec3::new(x, y, z).normalize();
        let up_dir = glam::Vec3::Z;
        let right_dir = look_dir.cross(up_dir).normalize();

        self.look_dir = look_dir;
        self.right_dir = right_dir;
        self.up_dir = up_dir;

        // Use wasd to move the camera
        if !self.lock_view {
            let move_speed = if input_state.is_key_down(KeyboardKey::LShift)
                || input_state.is_key_down(KeyboardKey::RShift)
            {
                FAST_MOVE_SPEED
            } else {
                NORMAL_MOVE_SPEED
            };

            //+x = forward
            //+y = right
            let mut velocity = glam::Vec3::default();
            if input_state.is_key_down(KeyboardKey::W) {
                velocity.x += move_speed;
            }

            if input_state.is_key_down(KeyboardKey::S) {
                velocity.x -= move_speed;
            }

            if input_state.is_key_down(KeyboardKey::A) {
                velocity.y -= move_speed;
            }

            if input_state.is_key_down(KeyboardKey::D) {
                velocity.y += move_speed;
            }

            self.position += velocity.x * self.look_dir * time_state.previous_update_dt();
            self.position += velocity.y * self.right_dir * time_state.previous_update_dt();
        }

        //println!("move speed {:?}", velocity);
        //println!("mouse delta {:?}", input_state.mouse_position_delta())
        //println!("pitch: {:?} yaw: {:?} velocity: {:?}", pitch_dt, yaw_dt, velocity);
        //println!("pitch: {:?} yaw: {:?} velocity: {:?}", self.pitch, self.yaw, self.position);
        //println!("yaw: {} pitch: {} look: {:?} up: {:?} right: {:?}", self.yaw.to_degrees(), self.pitch.to_degrees(), look_dir, up_dir, right_dir);
        // println!(
        //     "pos: {} pitch: {} yaw: {}",
        //     self.position, self.pitch, self.yaw
        // );
    }
}
