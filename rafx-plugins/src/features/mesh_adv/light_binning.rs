use crate::shaders::mesh_adv::{lights_bin_comp, lights_build_lists_comp};
use rafx::api::{RafxBufferDef, RafxMemoryUsage, RafxQueueType, RafxResourceType};
use rafx::framework::{BufferResource, ResourceArc, ResourceContext, MAX_FRAMES_IN_FLIGHT};
use rafx::RafxResult;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct LightBinAABB {
    _value: lights_bin_comp::ClusterAABBBuffer,
}

impl LightBinAABB {
    fn new(
        min: glam::Vec3,
        max: glam::Vec3,
    ) -> Self {
        LightBinAABB {
            _value: lights_bin_comp::ClusterAABBBuffer {
                min: min.into(),
                max: max.into(),
                _padding0: Default::default(),
                _padding1: Default::default(),
            },
        }
    }
}

#[derive(Clone)]
pub struct LightBinningFrustumConfig {
    pub near_z: f32,
    pub far_z: f32,
    pub x_bins: u32,
    pub y_bins: u32,
    pub z_bins: u32,
    //NOTE: We assume a "typical" perspective or orthographic projection.
    pub projection_matrix: glam::Mat4,
}

pub struct LightBinningFrustumAABBStructureInner {
    pub config: LightBinningFrustumConfig,
    pub aabb_list: Vec<LightBinAABB>,
}

#[derive(Clone)]
pub struct LightBinningFrustumAABBStructure {
    inner: Arc<LightBinningFrustumAABBStructureInner>,
}

impl Deref for LightBinningFrustumAABBStructure {
    type Target = LightBinningFrustumAABBStructureInner;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl LightBinningFrustumAABBStructure {
    pub fn get_linear_index(
        &self,
        x: u32,
        y: u32,
        z: u32,
    ) -> u32 {
        (self.config.x_bins * self.config.y_bins * z) + (self.config.x_bins * y) + x
    }

    pub fn get_cluster_by_index(
        &self,
        x: u32,
        y: u32,
        z: u32,
    ) -> &LightBinAABB {
        &self.aabb_list[self.get_linear_index(x, y, z) as usize]
    }

    pub fn new(config: &LightBinningFrustumConfig) -> Self {
        log::info!("Rebuilding LightBinningFrustumAABBStructure");

        // Divide the z depth into pieces that grow logarithmically (see idtech6 siggraph slides)
        let mut z_divisions = Vec::with_capacity(config.z_bins as usize - 1);
        z_divisions.push(0.0);
        let far_over_near = config.far_z / config.near_z;
        for i in 0..config.z_bins {
            z_divisions.push(
                -1.0 * config.near_z * far_over_near.powf(i as f32 / (config.z_bins as f32 - 1.0)),
            );
        }
        println!("desired z divisions: {:?}", z_divisions);

        // Validation for math in shader that determines cluster using z depth
        for i in 0..160 {
            let view_depth = -2.0_f32.powf(i as f32 / 10.0);
            let top = (config.z_bins - 1) as f32 * (-view_depth / config.near_z).ln();
            let bottom = (config.far_z / config.near_z).ln();

            let result = (((top / bottom) + 1.0).clamp(0.0, (config.z_bins - 1) as f32)) as u32;
            assert!(view_depth.max(-config.far_z + 0.001) > z_divisions[result as usize + 1]);
            assert!(view_depth <= z_divisions[result as usize]);
            println!(
                "{} = {} {} [{}, {}]",
                view_depth,
                top / bottom,
                result,
                z_divisions[result as usize],
                z_divisions[result as usize + 1]
            );
        }

        let mut aabb_list =
            Vec::with_capacity((config.z_bins * config.y_bins * config.x_bins) as usize);

        // Invert the given projection and find where the (1, 1, 0.1) and (1, 1, 0.9) map to in view space.
        // We just want to determine a slope and intersection with z plane, and an infinite projection
        // cannot be evaluated at z=1.0
        let projection_inv = config.projection_matrix.inverse();
        let mut projected_ray_near = projection_inv * glam::Vec4::new(1.0, 1.0, 0.1, 1.0);
        let mut projected_ray_far = projection_inv * glam::Vec4::new(1.0, 1.0, 0.9, 1.0);
        projected_ray_near /= projected_ray_near.w;
        projected_ray_far /= projected_ray_far.w;

        println!("near {:?} far {:?}", projected_ray_near, projected_ray_far);

        // Determine the slope and z plane intersect between the near/far positions
        use glam::Vec4Swizzles;
        let slope = (projected_ray_far.xy() - projected_ray_near.xy())
            / (projected_ray_far.z - projected_ray_near.z);
        let offset = projected_ray_near.xy() + (0.0 - projected_ray_near.z) * slope;

        println!("SLOPE: {:?} OFFSET {:?}", slope, offset);

        // Iterate across clusters in the top-right quadrant (we can mirror results to other quadrants)
        // x0,x1 and y0,y1 will be index of z division. We iterate across clusters between the
        // divisions
        for z0 in 0..config.z_bins {
            for y0 in 0..config.y_bins {
                for x0 in 0..config.x_bins {
                    let x1 = x0 + 1;
                    let y1 = y0 + 1;
                    let z1 = z0 + 1;

                    let near_z = z_divisions[z0 as usize];
                    let far_z = z_divisions[z1 as usize];

                    let xy0_frac = glam::Vec2::new(
                        x0 as f32 / config.x_bins as f32,
                        y0 as f32 / config.y_bins as f32,
                    ) * 2.0
                        - glam::Vec2::splat(1.0);
                    let xy1_frac = glam::Vec2::new(
                        x1 as f32 / config.x_bins as f32,
                        y1 as f32 / config.y_bins as f32,
                    ) * 2.0
                        - glam::Vec2::splat(1.0);

                    let xy0_at_unit_depth = xy0_frac * slope;
                    let xy1_at_unit_depth = xy1_frac * slope;

                    let min_max_x = if xy0_frac.x < 0.0 {
                        (far_z * xy0_at_unit_depth.x, near_z * xy1_at_unit_depth.x)
                    } else {
                        (near_z * xy0_at_unit_depth.x, far_z * xy1_at_unit_depth.x)
                    };

                    let min_max_y = if xy0_frac.y < 0.0 {
                        (far_z * xy0_at_unit_depth.y, near_z * xy1_at_unit_depth.y)
                    } else {
                        (near_z * xy0_at_unit_depth.y, far_z * xy1_at_unit_depth.y)
                    };

                    let xy0_offset = xy0_frac * offset;
                    let xy1_offset = xy1_frac * offset;
                    let offset_min = glam::Vec3::new(
                        xy0_offset.x.min(xy1_offset.x),
                        xy0_offset.y.min(xy1_offset.y),
                        0.0,
                    );
                    let offset_max = glam::Vec3::new(
                        xy0_offset.x.max(xy1_offset.x),
                        xy0_offset.y.max(xy1_offset.y),
                        0.0,
                    );

                    let min_vs = glam::Vec3::new(min_max_x.0, min_max_y.0, 1.0 * far_z)
                        - glam::Vec3::splat(0.01)
                        + offset_min;
                    let max_vs = glam::Vec3::new(min_max_x.1, min_max_y.1, 1.0 * near_z)
                        + glam::Vec3::splat(0.01)
                        + offset_max;

                    aabb_list.push(LightBinAABB::new(min_vs, max_vs));
                }
            }
        }

        let inner = LightBinningFrustumAABBStructureInner {
            aabb_list,
            config: config.clone(),
        };

        LightBinningFrustumAABBStructure {
            inner: Arc::new(inner),
        }
    }
}

pub struct MeshAdvLightBinRenderResource {
    frustum_structure: Option<LightBinningFrustumAABBStructure>,
    // Unlikely to change often so we allocate/drop as needed
    frustum_bounds_gpu_buffer: Option<ResourceArc<BufferResource>>,
    // Uploaded by CPU, so MAX_FRAMES_IN_FLIGHT + 1 buffers needed
    light_bounds_gpu_buffers: Vec<ResourceArc<BufferResource>>,
    // GPU-only buffers, MAX_FRAMES_IN_FLIGHT buffers needed
    output_gpu_buffers: Vec<ResourceArc<BufferResource>>,
}

impl MeshAdvLightBinRenderResource {
    pub fn new(resource_context: &ResourceContext) -> RafxResult<Self> {
        // One for CPU to write + GPU frames in flight
        let mut light_bounds_gpu_buffers = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT + 1);
        for _ in 0..=MAX_FRAMES_IN_FLIGHT {
            light_bounds_gpu_buffers.push(
                resource_context.resources().insert_buffer(
                    resource_context
                        .device_context()
                        .create_buffer(&RafxBufferDef {
                            size: std::mem::size_of::<lights_bin_comp::LightsInputListBuffer>()
                                as u64,
                            alignment: 256,
                            memory_usage: RafxMemoryUsage::CpuToGpu,
                            queue_type: RafxQueueType::Graphics,
                            resource_type: RafxResourceType::BUFFER,
                            ..Default::default()
                        })?,
                ),
            );
        }

        // One per GPU frame in flight (CPU doesn't modify it)
        let mut output_gpu_buffers = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT + 1);
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            output_gpu_buffers.push(
                resource_context.resources().insert_buffer(
                    resource_context
                        .device_context()
                        .create_buffer(&RafxBufferDef {
                            size: std::mem::size_of::<
                                lights_build_lists_comp::LightBuildListsOutputBuffer,
                            >() as u64,
                            alignment: 256,
                            memory_usage: RafxMemoryUsage::GpuOnly,
                            queue_type: RafxQueueType::Graphics,
                            resource_type: RafxResourceType::BUFFER_READ_WRITE,
                            ..Default::default()
                        })?,
                ),
            );
        }

        Ok(MeshAdvLightBinRenderResource {
            frustum_structure: None,
            frustum_bounds_gpu_buffer: None,
            light_bounds_gpu_buffers,
            output_gpu_buffers,
        })
    }

    pub fn update_projection(
        &mut self,
        resource_context: &ResourceContext,
        projection_matrix: &glam::Mat4,
    ) -> RafxResult<()> {
        if self.frustum_structure.is_none()
            || !self
                .frustum_structure
                .as_ref()
                .unwrap()
                .config
                .projection_matrix
                .abs_diff_eq(*projection_matrix, 0.0001)
        {
            self.frustum_structure = Some(LightBinningFrustumAABBStructure::new(
                &LightBinningFrustumConfig {
                    near_z: 5.0,
                    far_z: 10000.0,
                    x_bins: 16,
                    y_bins: 8,
                    z_bins: 24,
                    projection_matrix: projection_matrix.clone(),
                },
            ));

            // We assume the cluster data starts at offset 0
            assert_eq!(
                memoffset::offset_of!(lights_bin_comp::BinLightsConfigStd430, clusters),
                0
            );
            // We assume that LightBinAABB and ClusterAABBBuffer are the same length (LightBinAABB should be repr(transparent))
            assert_eq!(
                std::mem::size_of::<LightBinAABB>(),
                std::mem::size_of::<lights_bin_comp::ClusterAABBBuffer>()
            );
            // We assume the aabb list is short enough to fit into the shader's array
            assert!(
                std::mem::size_of::<lights_bin_comp::BinLightsConfigStd430>()
                    >= self.frustum_structure.as_ref().unwrap().aabb_list.len()
                        * std::mem::size_of::<LightBinAABB>()
            );

            let allocator = resource_context.create_dyn_resource_allocator_set();
            let frustum_bounds_gpu_buffer =
                allocator.insert_buffer(resource_context.device_context().create_buffer(
                    &RafxBufferDef {
                        size: std::mem::size_of::<lights_bin_comp::BinLightsConfigBuffer>() as u64,
                        alignment: 256,
                        memory_usage: RafxMemoryUsage::CpuToGpu,
                        queue_type: RafxQueueType::Graphics,
                        resource_type: RafxResourceType::BUFFER,
                        ..Default::default()
                    },
                )?);

            frustum_bounds_gpu_buffer
                .get_raw()
                .buffer
                .copy_to_host_visible_buffer(
                    &*self.frustum_structure.as_ref().unwrap().aabb_list,
                )?;

            self.frustum_bounds_gpu_buffer = Some(frustum_bounds_gpu_buffer);

            //TODO: Should this be copied to a device local buffer?
        }

        Ok(())
    }

    pub fn update_light_bounds(
        &self,
        frame_index: usize,
        lights: &lights_bin_comp::LightsInputListBuffer,
    ) -> RafxResult<()> {
        self.light_bounds_gpu_buffers[frame_index % (MAX_FRAMES_IN_FLIGHT + 1)]
            .get_raw()
            .buffer
            .copy_to_host_visible_buffer(&[*lights])
    }

    pub fn aabb_structure(&self) -> &Option<LightBinningFrustumAABBStructure> {
        &self.frustum_structure
    }

    pub fn frustum_bounds_gpu_buffer(&self) -> &Option<ResourceArc<BufferResource>> {
        &self.frustum_bounds_gpu_buffer
    }

    pub fn light_bounds_gpu_buffer(
        &self,
        frame_index: usize,
    ) -> &ResourceArc<BufferResource> {
        &self.light_bounds_gpu_buffers[frame_index % (MAX_FRAMES_IN_FLIGHT + 1)]
    }

    pub fn output_gpu_buffer(
        &self,
        frame_index: usize,
    ) -> &ResourceArc<BufferResource> {
        &self.output_gpu_buffers[frame_index % MAX_FRAMES_IN_FLIGHT]
    }
}
