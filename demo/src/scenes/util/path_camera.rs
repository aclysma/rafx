use crate::input::InputState;
use crate::TimeState;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone)]
pub struct PathData {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
}

pub struct PathCamera {
    pub position: glam::Vec3,
    pub look_dir: glam::Vec3,
    pub right_dir: glam::Vec3,
    pub up_dir: glam::Vec3,
    pub path_data: Vec<PathData>,
    pub time: Duration,
}

impl PathCamera {
    pub fn new(path_data: Vec<PathData>) -> Self {
        PathCamera {
            position: glam::Vec3::ZERO,
            look_dir: glam::Vec3::X,
            right_dir: -glam::Vec3::Y,
            up_dir: glam::Vec3::Z,
            path_data,
            time: Duration::ZERO,
        }
    }

    pub fn update(
        &mut self,
        _input_state: &InputState,
        time_state: &TimeState,
    ) {
        self.time += time_state.previous_update_time() * 20;
        let mut frame_time = self.time.as_secs_f32();
        while frame_time > self.path_data.len() as f32 {
            frame_time -= self.path_data.len() as f32;
        }

        let frame_index0 = frame_time
            .floor()
            .clamp(0.0, self.path_data.len() as f32 - 1.0);
        let frame_index1 = frame_time
            .ceil()
            .clamp(0.0, self.path_data.len() as f32 - 1.0);
        let t = if frame_index1 > frame_index0 {
            (frame_time - frame_index0) / (frame_index1 - frame_index0)
        } else {
            frame_index1
        };

        let p0: glam::Vec3 = self.path_data[frame_index0 as usize].position.into();
        let p1: glam::Vec3 = self.path_data[frame_index1 as usize].position.into();
        let q0: glam::Quat = self.path_data[frame_index0 as usize].rotation.into();
        let q1: glam::Quat = self.path_data[frame_index1 as usize].rotation.into();
        self.position = p0.lerp(p1, t);
        let rotation = q0.slerp(q1, t);
        self.look_dir = rotation.mul_vec3(-glam::Vec3::Z).normalize();
        self.up_dir = glam::Vec3::Z;
        self.right_dir = self.look_dir.cross(self.up_dir).normalize();
    }
}
