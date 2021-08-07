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

pub struct Debug3DResource {
    line_lists: Vec<LineList3D>,
}

impl Debug3DResource {
    pub fn new() -> Self {
        Debug3DResource { line_lists: vec![] }
    }

    pub fn add_line_strip(
        &mut self,
        points: Vec<glam::Vec3>,
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
            points.push(points[0]);
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
        if normal.dot(glam::Vec3::Z).abs() > 0.9999 {
            // Can't cross the Z axis with the up vector, so special case that here
            (glam::Vec3::X, glam::Vec3::Y)
        } else {
            let x_dir = normal.cross(glam::Vec3::Z);
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
        // Draw the vertical rings
        for index in 0..segments {
            // Rotate around whole sphere (2pi)
            let fraction = (index as f32 / segments as f32) * std::f32::consts::PI * 2.0;
            let x_dir = glam::Vec3::new(fraction.cos(), fraction.sin(), 0.0);
            let y_dir = glam::Vec3::Z;

            self.add_circle_xy(center, x_dir, y_dir, radius, color, segments);
        }

        // Draw the center horizontal ring
        self.add_circle_xy(
            center,
            glam::Vec3::X,
            glam::Vec3::Y,
            radius,
            color,
            segments,
        );

        // Draw the off-center horizontal rings
        for index in 1..(segments / 2) {
            let fraction = (index as f32 / segments as f32) * std::f32::consts::PI * 2.0;

            let r = radius * fraction.cos();
            let z_offset = radius * fraction.sin() * glam::Vec3::Z;

            //let transform = glam::Mat4::from_translation(center + glam::Vec3::new(0.0, 0.0, z_offset));
            self.add_circle_xy(
                center + z_offset,
                glam::Vec3::X,
                glam::Vec3::Y,
                r,
                color,
                segments,
            );

            self.add_circle_xy(
                center - z_offset,
                glam::Vec3::X,
                glam::Vec3::Y,
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
            let fraction = index as f32 / segments as f32;

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

    pub fn add_axis_aligned_grid(
        &mut self,
        step_distance: f32,
    ) {
        const GRID_LINE_AXIS_COLOR_INTENSITY: f32 = 0.25;
        const GRID_LINE_MAJOR_STEP_COLOR_INTENSITY: f32 = 0.1;
        const GRID_LINE_MINOR_STEP_COLOR_INTENSITY: f32 = 0.025;
        const GRID_LINE_MAJOR_STEP_COUNT: i32 = 10;
        const GRID_LINE_MINOR_STEP_COUNT: i32 = 10;
        const GRID_LINE_TOTAL_STEP_COUNT: i32 =
            GRID_LINE_MAJOR_STEP_COUNT * GRID_LINE_MINOR_STEP_COUNT;

        let grid_center = glam::Vec3::ZERO;
        for i in -GRID_LINE_TOTAL_STEP_COUNT..=GRID_LINE_TOTAL_STEP_COUNT {
            if i == 0 {
                continue;
            }

            let line_color = if (i.abs() % GRID_LINE_MINOR_STEP_COUNT) == 0 {
                glam::Vec3::splat(GRID_LINE_MAJOR_STEP_COLOR_INTENSITY).extend(1.0)
            } else {
                glam::Vec3::splat(GRID_LINE_MINOR_STEP_COLOR_INTENSITY).extend(1.0)
            };

            let offset_y = glam::Vec3::Y * (i as f32 * step_distance) + grid_center;
            self.add_line(
                glam::Vec3::X * -1000.0 + offset_y,
                glam::Vec3::X * 1000.0 + offset_y,
                line_color,
            );

            let offset_x = glam::Vec3::X * (i as f32 * step_distance) + grid_center;
            self.add_line(
                glam::Vec3::Y * -1000.0 + offset_x,
                glam::Vec3::Y * 1000.0 + offset_x,
                line_color,
            );
        }

        self.add_line(
            glam::Vec3::X * -1000.0 + grid_center,
            glam::Vec3::X * 1000.0 + grid_center,
            (glam::Vec3::X * GRID_LINE_AXIS_COLOR_INTENSITY).extend(1.0),
        );
        self.add_line(
            glam::Vec3::Y * -1000.0 + grid_center,
            glam::Vec3::Y * 1000.0 + grid_center,
            (glam::Vec3::Y * GRID_LINE_AXIS_COLOR_INTENSITY).extend(1.0),
        );
        self.add_line(
            glam::Vec3::Z * -1000.0 + grid_center,
            glam::Vec3::Z * 1000.0 + grid_center,
            (glam::Vec3::Z * GRID_LINE_AXIS_COLOR_INTENSITY).extend(1.0),
        );
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
