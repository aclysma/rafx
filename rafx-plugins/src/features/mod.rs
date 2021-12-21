pub mod debug3d;
pub mod skybox;
pub mod text;
pub mod tile_layer;

// This feature currently requires legion
#[cfg(feature = "legion")]
pub mod sprite;

// This feature currently requires legion
#[cfg(all(feature = "basic-pipeline", feature = "legion"))]
pub mod mesh_basic;

#[cfg(all(feature = "modern-pipeline", feature = "legion"))]
pub mod mesh_adv;

#[cfg(feature = "egui")]
pub mod egui;

#[cfg(feature = "use-imgui")]
pub mod imgui;
