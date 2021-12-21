mod gltf_importer;

use glam::{Vec2, Vec3};

pub use gltf_importer::*;

mod blender_material_importer;
pub use blender_material_importer::*;

mod blender_model_importer;
pub use blender_model_importer::*;

mod blender_mesh_importer;
pub use blender_mesh_importer::*;

mod blender_prefab_importer;
pub use blender_prefab_importer::*;

use super::assets::*;

// Calculates a tangent and binormal that are orthogonal to the polygon normal and align with the
// uv coordinate x and y axes respectively. May return zero vector if uv coordinates are the same
fn calculate_tangent_binormal(
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
    uv0: Vec2,
    uv1: Vec2,
    uv2: Vec2,
) -> (Vec3, Vec3) {
    let dp1 = p1 - p0;
    let dp2 = p2 - p0;

    let duv1 = uv1 - uv0;
    let duv2 = uv2 - uv0;

    let denominator = duv1.x * duv2.y - duv1.y * duv2.x;
    let (t, b) = if denominator.abs() > 0.0001 {
        let r = 1.0 / denominator;
        let t = (dp1 * duv2.y - dp2 * duv1.y) * r;
        let b = (dp2 * duv1.x - dp1 * duv2.x) * r;

        // We normalize, weighted by the size of the triangle. So we can just add all the
        // tangents/binormals and normalize it at the end
        (t, b)
    } else {
        // If not divisible, assume uv coordinates don't change and the tangent/binormal
        // won't matter
        (glam::Vec3::ZERO, glam::Vec3::ZERO)
    };
    (t, b)
}

// Given unnormalized and possibly invalid tangent/binormals, produce a tangent/binormal that is
// normalized and appropriate for right-hand coordinate system.
fn fix_tangent_binormal(
    n: Vec3,
    t: Vec3,
    b: Vec3,
) -> (Vec3, Vec3) {
    let mut t = if t.length_squared() > 0.0001 {
        t.normalize()
    } else {
        // If we don't have a valid vector to normalize, just pick anything
        n.any_orthogonal_vector()
    };

    let b = if b.length_squared() > 0.0001 {
        let b = b.normalize();

        // Ensure right-handed coordinate system.. flip the tangent if it's not
        // See handedness section in
        // https://www.opengl-tutorial.org/intermediate-tutorials/tutorial-13-normal-mapping/
        if n.cross(t).dot(b) < 0.0 {
            t = t * -1.0;
        }

        b
    } else {
        // If we don't have a valid vector to normalize, pick anything orthogonal
        // to normal and tangent
        n.cross(t).normalize()
    };

    (t, b)
}
