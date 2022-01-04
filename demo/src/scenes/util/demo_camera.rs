use crate::input::{InputState, KeyboardKey};
use crate::scenes::util::{FlyCamera, PathCamera, PathData};
use crate::TimeState;
use glam::Vec3Swizzles;

#[derive(PartialEq)]
pub enum CameraMode {
    Fly,
    Path,
}

pub struct DemoCamera {
    pub fly_camera: FlyCamera,
    pub path_camera: Option<PathCamera>,
    pub mode: CameraMode,
}

impl DemoCamera {
    pub fn new() -> Self {
        DemoCamera {
            fly_camera: FlyCamera::default(),
            path_camera: None,
            mode: CameraMode::Fly,
        }
    }

    pub fn new_with_path(path_data: Vec<PathData>) -> Self {
        DemoCamera {
            fly_camera: FlyCamera::default(),
            path_camera: Some(PathCamera::new(path_data)),
            mode: CameraMode::Path,
        }
    }

    pub fn update(
        &mut self,
        input_state: &InputState,
        time_state: &TimeState,
    ) {
        if input_state.is_key_just_down(KeyboardKey::T) {
            self.mode = CameraMode::Path
        };

        if input_state.is_key_just_down(KeyboardKey::F) {
            if self.mode != CameraMode::Fly {
                // This is dumb.. but the fly camera will also catch the F down event and toggle
                // this.
                self.fly_camera.lock_view = false;

                if self.path_camera.is_some() {
                    let path_camera = self.path_camera.as_ref().unwrap();
                    self.fly_camera.position = path_camera.position;
                    println!(
                        "Fly to Path camera {:?} {}",
                        path_camera.look_dir,
                        path_camera.look_dir.z.asin()
                    );
                    self.fly_camera.pitch =
                        (path_camera.look_dir.z / path_camera.look_dir.xy().length()).atan();
                    self.fly_camera.yaw = path_camera.look_dir.y.atan2(path_camera.look_dir.x);
                }
            }
            self.mode = CameraMode::Fly;
        }

        match self.mode {
            CameraMode::Fly => {
                self.fly_camera.update(input_state, time_state);
            }
            CameraMode::Path => {
                if let Some(path_camera) = &mut self.path_camera {
                    path_camera.update(input_state, time_state);
                }
            }
        }
    }

    pub fn position(&self) -> glam::Vec3 {
        match self.mode {
            CameraMode::Fly => self.fly_camera.position,
            CameraMode::Path => {
                if let Some(path_camera) = &self.path_camera {
                    path_camera.position
                } else {
                    glam::Vec3::ZERO
                }
            }
        }
    }

    pub fn look_dir(&self) -> glam::Vec3 {
        match self.mode {
            CameraMode::Fly => self.fly_camera.look_dir,
            CameraMode::Path => {
                if let Some(path_camera) = &self.path_camera {
                    path_camera.look_dir
                } else {
                    glam::Vec3::X
                }
            }
        }
    }

    pub fn right_dir(&self) -> glam::Vec3 {
        match self.mode {
            CameraMode::Fly => self.fly_camera.right_dir,
            CameraMode::Path => {
                if let Some(path_camera) = &self.path_camera {
                    path_camera.right_dir
                } else {
                    glam::Vec3::Y
                }
            }
        }
    }

    pub fn up_dir(&self) -> glam::Vec3 {
        match self.mode {
            CameraMode::Fly => self.fly_camera.up_dir,
            CameraMode::Path => {
                if let Some(path_camera) = &self.path_camera {
                    path_camera.up_dir
                } else {
                    glam::Vec3::Z
                }
            }
        }
    }
}
