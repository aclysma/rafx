use crate::render_contexts::{RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext};
use atelier_assets::loader::handle::Handle;
use std::sync::atomic::{AtomicI32, Ordering};
use renderer::nodes::DefaultExtractJob;
use crate::features::debug3d::extract::Debug3dExtractJobImpl;
use renderer::vulkan::VkDeviceContext;
use renderer::resources::DescriptorSetAllocatorRef;
use renderer::resources::PipelineSwapchainInfo;
use renderer::assets::MaterialAsset;
use renderer::nodes::ExtractJob;
use renderer::nodes::RenderFeature;
use renderer::nodes::RenderFeatureIndex;
use renderer::resources::DescriptorSetArc;
use std::convert::TryInto;

mod extract;
mod prepare;
mod write;

pub struct LineList3D {
    pub points: Vec<glam::Vec3>,
    pub color: glam::Vec4,
}

impl LineList3D {
    pub fn new(
        points: Vec<glam::Vec3>,
        color: glam::Vec4,
    ) -> Self {
        LineList3D { points, color }
    }
}

pub struct DebugDraw3DResource {
    line_lists: Vec<LineList3D>,
}

impl DebugDraw3DResource {
    pub fn new() -> Self {
        DebugDraw3DResource { line_lists: vec![] }
    }

    pub fn add_line_strip(
        &mut self,
        mut points: Vec<glam::Vec3>,
        color: glam::Vec4,
    ) {
        // Nothing will draw if we don't have at least 2 points
        if points.len() > 1 {
            self.line_lists.push(LineList3D::new(points, color));
        }
    }

    // Adds a single polygon
    pub fn add_line_loop(
        &mut self,
        mut points: Vec<glam::Vec3>,
        color: glam::Vec4,
    ) {
        // Nothing will draw if we don't have at least 2 points
        if points.len() > 1 {
            points.push(points[0].clone());
            self.add_line_strip(points, color);
        }
    }

    pub fn add_line(
        &mut self,
        p0: glam::Vec3,
        p1: glam::Vec3,
        color: glam::Vec4,
    ) {
        let points = vec![p0, p1];
        self.add_line_strip(points, color);
    }

    // Takes an X/Y axis pair and center position
    pub fn add_circle_xy(
        &mut self,
        center: glam::Vec3,
        x_dir: glam::Vec3,
        y_dir: glam::Vec3,
        radius: f32,
        color: glam::Vec4,
        segments: u32,
    ) {
        let x_dir = x_dir * radius;
        let y_dir = y_dir * radius;

        let mut points = Vec::with_capacity(segments as usize + 1);
        for index in 0..segments {
            let fraction = (index as f32 / segments as f32) * std::f32::consts::PI * 2.0;

            //let position = glam::Vec4::new(fraction.sin() * radius, fraction.cos() * radius, 0.0, 1.0);
            //let transformed = transform * position;
            points.push(center + (fraction.cos() * x_dir) + (fraction.sin() * y_dir));
        }

        self.add_line_loop(points, color);
    }

    pub fn normal_to_xy(normal: glam::Vec3) -> (glam::Vec3, glam::Vec3) {
        if normal.dot(glam::Vec3::unit_z()).abs() > 0.9999 {
            // Can't cross the Z axis with the up vector, so special case that here
            (glam::Vec3::unit_x(), glam::Vec3::unit_y())
        } else {
            let x_dir = normal.cross(glam::Vec3::unit_z());
            let y_dir = x_dir.cross(normal);
            (x_dir, y_dir)
        }
    }

    // Takes a normal and center position
    pub fn add_circle(
        &mut self,
        center: glam::Vec3,
        normal: glam::Vec3,
        radius: f32,
        color: glam::Vec4,
        segments: u32,
    ) {
        let (x_dir, y_dir) = Self::normal_to_xy(normal);
        self.add_circle_xy(center, x_dir, y_dir, radius, color, segments);
    }

    pub fn add_sphere(
        &mut self,
        center: glam::Vec3,
        radius: f32,
        color: glam::Vec4,
        segments: u32,
    ) {
        let world_tranform = glam::Mat4::from_translation(center);

        // Draw the vertical rings
        for index in 0..segments {
            // Rotate around whole sphere (2pi)
            let fraction = (index as f32 / segments as f32) * std::f32::consts::PI * 2.0;
            let x_dir = glam::Vec3::new(fraction.cos(), fraction.sin(), 0.0);
            let y_dir = glam::Vec3::unit_z();

            self.add_circle_xy(center, x_dir, y_dir, radius, color, segments);
        }

        // Draw the center horizontal ring
        self.add_circle_xy(
            center,
            glam::Vec3::unit_x(),
            glam::Vec3::unit_y(),
            radius,
            color,
            segments,
        );

        // Draw the off-center horizontal rings
        for index in 1..(segments / 2) {
            let fraction = (index as f32 / segments as f32) * std::f32::consts::PI * 2.0;

            let r = radius * fraction.cos();
            let z_offset = radius * fraction.sin() * glam::Vec3::unit_z();

            //let transform = glam::Mat4::from_translation(center + glam::Vec3::new(0.0, 0.0, z_offset));
            self.add_circle_xy(
                center + z_offset,
                glam::Vec3::unit_x(),
                glam::Vec3::unit_y(),
                r,
                color,
                segments,
            );

            self.add_circle_xy(
                center - z_offset,
                glam::Vec3::unit_x(),
                glam::Vec3::unit_y(),
                r,
                color,
                segments,
            );
        }
    }

    pub fn add_cone(
        &mut self,
        vertex: glam::Vec3,      // (position of the pointy bit)
        base_center: glam::Vec3, // (position of the center of the base of the cone)
        radius: f32,
        color: glam::Vec4,
        segments: u32,
    ) {
        let base_to_vertex = vertex - base_center;
        let base_to_vertex_normal = base_to_vertex.normalize();
        let (x_dir, y_dir) = Self::normal_to_xy(base_to_vertex_normal);
        for index in 0..segments {
            let fraction = (index as f32 / segments as f32);

            let center = base_center + base_to_vertex * fraction;
            self.add_circle_xy(
                center,
                x_dir,
                y_dir,
                radius * (1.0 - fraction),
                color,
                segments,
            );
        }

        for index in 0..segments / 2 {
            let fraction = (index as f32 / (segments / 2) as f32) * std::f32::consts::PI;
            let offset = ((x_dir * fraction.cos()) + (y_dir * fraction.sin())) * radius;

            let p0 = base_center + offset;
            let p1 = vertex;
            let p2 = base_center - offset;
            self.add_line_strip(vec![p0, p1, p2], color);
        }
    }

    // Returns the draw data, leaving this object in an empty state
    pub fn take_line_lists(&mut self) -> Vec<LineList3D> {
        std::mem::replace(&mut self.line_lists, vec![])
    }

    // Recommended to call every frame to ensure that this doesn't grow unbounded
    pub fn clear(&mut self) {
        self.line_lists.clear();
    }
}


pub fn create_debug3d_extract_job(
    device_context: VkDeviceContext,
    descriptor_set_allocator: DescriptorSetAllocatorRef,
    pipeline_info: PipelineSwapchainInfo,
    debug3d_material: &Handle<MaterialAsset>,
) -> Box<dyn ExtractJob<RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext>> {
    Box::new(Debug3dExtractJobImpl::new(
        device_context,
        descriptor_set_allocator,
        pipeline_info,
        debug3d_material,
    ))
}

/// Per-pass "global" data
#[derive(Clone, Debug, Copy)]
struct Debug3dUniformBufferObject {
    // View and projection matrices
    view_proj: [[f32; 4]; 4],
}

/// Vertex format for vertices sent to the GPU
#[derive(Clone, Debug, Copy)]
#[repr(C)]
pub struct Debug3dVertex {
    pub pos: [f32; 3],
    //pub tex_coord: [f32; 2],
    pub color: [f32; 4],
}

//
// This is boilerplate that could be macro'd
//
static DEBUG_3D_FEATURE_INDEX: AtomicI32 = AtomicI32::new(-1);

pub struct Debug3dRenderFeature;

impl RenderFeature for Debug3dRenderFeature {
    fn set_feature_index(index: RenderFeatureIndex) {
        DEBUG_3D_FEATURE_INDEX.store(index.try_into().unwrap(), Ordering::Release);
    }

    fn feature_index() -> RenderFeatureIndex {
        DEBUG_3D_FEATURE_INDEX.load(Ordering::Acquire) as RenderFeatureIndex
    }

    fn feature_debug_name() -> &'static str {
        "Debug3dRenderFeature"
    }
}

pub(self) struct ExtractedDebug3dData {
    // position: glam::Vec3,
    // texture_size: glam::Vec2,
    // scale: f32,
    // rotation: f32,
    // alpha: f32,
    // texture_descriptor_set: DescriptorSetArc, //TODO: I'd prefer to use something ref-counted
    line_lists: Vec<LineList3D>
}

// #[derive(Debug)]
// pub struct Debug3dDrawCall {
//     index_buffer_first_element: u16,
//     index_buffer_count: u16,
//     texture_descriptor_set: DescriptorSetArc,
// }

#[derive(Debug)]
struct Debug3dDrawCall {
    first_element: u32,
    count: u32,
}