use crate::metal::{
    ArgumentBufferData, BarrierFlagsMetal, RafxBufferMetal, RafxCommandPoolMetal,
    RafxDescriptorSetArrayMetal, RafxDescriptorSetHandleMetal, RafxPipelineMetal, RafxQueueMetal,
    RafxRenderTargetMetal, RafxRootSignatureMetal, RafxTextureMetal,
};
use crate::{
    RafxBufferBarrier, RafxCmdBlitParams, RafxCmdCopyBufferToTextureParams,
    RafxColorRenderTargetBinding, RafxCommandBufferDef, RafxDepthRenderTargetBinding,
    RafxExtents3D, RafxIndexBufferBinding, RafxIndexType, RafxLoadOp, RafxPipelineType,
    RafxRenderTargetBarrier, RafxResourceState, RafxResult, RafxTextureBarrier,
    RafxVertexBufferBinding,
};
use fnv::FnvHashSet;
use metal_rs::{
    MTLBlitOption, MTLIndexType, MTLOrigin, MTLPrimitiveType, MTLRenderStages, MTLResourceUsage,
    MTLScissorRect, MTLSize, MTLViewport,
};
use rafx_base::trust_cell::TrustCell;

// Mutable state stored in a lock. (Hopefully we can optimize away the lock later)
#[derive(Debug)]
pub struct RafxCommandBufferMetalInner {
    render_targets_to_make_readable: FnvHashSet<RafxRenderTargetMetal>,
    command_buffer: Option<metal_rs::CommandBuffer>,
    render_encoder: Option<metal_rs::RenderCommandEncoder>,
    compute_encoder: Option<metal_rs::ComputeCommandEncoder>,
    blit_encoder: Option<metal_rs::BlitCommandEncoder>,
    current_index_buffer: Option<metal_rs::Buffer>,
    current_index_buffer_offset: u64,
    current_index_buffer_type: MTLIndexType,
    current_index_buffer_stride: u32,
    last_pipeline_type: Option<RafxPipelineType>,
    primitive_type: MTLPrimitiveType,
    current_render_targets_width: u32,
    current_render_targets_height: u32,
    compute_threads_per_group_x: u32,
    compute_threads_per_group_y: u32,
    compute_threads_per_group_z: u32,
}

unsafe impl Send for RafxCommandBufferMetalInner {}
unsafe impl Sync for RafxCommandBufferMetalInner {}

#[derive(Debug)]
pub struct RafxCommandBufferMetal {
    queue: RafxQueueMetal,
    inner: TrustCell<RafxCommandBufferMetalInner>,
}

impl RafxCommandBufferMetal {
    pub fn metal_command_buffer(&self) -> Option<&metal_rs::CommandBufferRef> {
        use foreign_types_shared::ForeignType;
        use foreign_types_shared::ForeignTypeRef;
        let ptr = self
            .inner
            .borrow()
            .command_buffer
            .as_ref()
            .map(|x| x.as_ptr());
        ptr.map(|x| unsafe { metal_rs::CommandBufferRef::from_ptr(x) })
    }

    pub fn metal_render_encoder(&self) -> Option<&metal_rs::RenderCommandEncoderRef> {
        use foreign_types_shared::ForeignType;
        use foreign_types_shared::ForeignTypeRef;
        let ptr = self
            .inner
            .borrow()
            .render_encoder
            .as_ref()
            .map(|x| x.as_ptr());
        ptr.map(|x| unsafe { metal_rs::RenderCommandEncoderRef::from_ptr(x) })
    }

    pub fn metal_compute_encoder(&self) -> Option<&metal_rs::ComputeCommandEncoderRef> {
        use foreign_types_shared::ForeignType;
        use foreign_types_shared::ForeignTypeRef;
        let ptr = self
            .inner
            .borrow()
            .compute_encoder
            .as_ref()
            .map(|x| x.as_ptr());
        ptr.map(|x| unsafe { metal_rs::ComputeCommandEncoderRef::from_ptr(x) })
    }

    pub fn metal_blit_encoder(&self) -> Option<&metal_rs::BlitCommandEncoderRef> {
        use foreign_types_shared::ForeignType;
        use foreign_types_shared::ForeignTypeRef;
        let ptr = self
            .inner
            .borrow()
            .blit_encoder
            .as_ref()
            .map(|x| x.as_ptr());
        ptr.map(|x| unsafe { metal_rs::BlitCommandEncoderRef::from_ptr(x) })
    }

    pub(crate) fn clear_command_buffer(&self) {
        self.inner.borrow_mut().command_buffer = None;
    }

    pub fn new(
        command_pool: &RafxCommandPoolMetal,
        _command_buffer_def: &RafxCommandBufferDef,
    ) -> RafxResult<RafxCommandBufferMetal> {
        let inner = RafxCommandBufferMetalInner {
            render_targets_to_make_readable: Default::default(),
            command_buffer: None,
            render_encoder: None,
            compute_encoder: None,
            blit_encoder: None,
            last_pipeline_type: None,
            primitive_type: MTLPrimitiveType::Triangle,
            current_render_targets_width: 0,
            current_render_targets_height: 0,
            compute_threads_per_group_x: 0,
            compute_threads_per_group_y: 0,
            compute_threads_per_group_z: 0,
            current_index_buffer: None,
            current_index_buffer_offset: 0,
            current_index_buffer_type: MTLIndexType::UInt16,
            current_index_buffer_stride: 0,
        };

        Ok(RafxCommandBufferMetal {
            queue: command_pool.queue().clone(),
            inner: TrustCell::new(inner),
        })
    }

    pub fn begin(&self) -> RafxResult<()> {
        objc::rc::autoreleasepool(|| {
            let command_buffer = self.queue.metal_queue().new_command_buffer();
            let mut inner = self.inner.borrow_mut();
            inner.command_buffer = Some(command_buffer.to_owned());
            inner.last_pipeline_type = None;
            Ok(())
        })
    }

    pub fn end(&self) -> RafxResult<()> {
        objc::rc::autoreleasepool(|| self.end_current_encoders(true))
    }

    pub fn return_to_pool(&self) -> RafxResult<()> {
        // Returning to pool means the command buffer no longer needs to stay valid, so drop the
        // current one
        self.inner.borrow_mut().command_buffer = None;
        Ok(())
    }

    pub fn cmd_bind_render_targets(
        &self,
        color_targets: &[RafxColorRenderTargetBinding],
        depth_target: Option<RafxDepthRenderTargetBinding>,
    ) -> RafxResult<()> {
        // if self.has_active_renderpass.load(Ordering::Relaxed) {
        //     self.cmd_unbind_render_targets()?;
        // }

        if color_targets.is_empty() && depth_target.is_none() {
            Err("No color or depth target supplied to cmd_bind_render_targets")?;
        }

        let mut extents = RafxExtents3D::default();

        let result: RafxResult<()> = objc::rc::autoreleasepool(|| {
            let descriptor = metal_rs::RenderPassDescriptor::new();

            let mut inner = self.inner.borrow_mut();
            for (i, color_target) in color_targets.iter().enumerate() {
                let color_descriptor = descriptor.color_attachments().object_at(i as _).unwrap();
                let texture = color_target
                    .render_target
                    .texture()
                    .metal_texture()
                    .unwrap();

                // Ensure current_render_targets_width/current_render_targets_depth are set
                extents = texture.texture_def().extents;
                inner.current_render_targets_width = extents.width;
                inner.current_render_targets_height = extents.height;

                color_descriptor.set_texture(Some(texture.metal_texture()));
                color_descriptor.set_level(color_target.mip_slice.unwrap_or(0) as _);
                if color_target.array_slice.is_some() {
                    if texture.texture_def().extents.depth > 1 {
                        color_descriptor.set_depth_plane(color_target.array_slice.unwrap() as _);
                    } else {
                        color_descriptor.set_slice(color_target.array_slice.unwrap() as _);
                    }
                }

                color_descriptor.set_load_action(color_target.load_op.into());
                let store_action =
                    super::util::color_render_target_binding_mtl_store_op(color_target);
                color_descriptor.set_store_action(store_action);

                if color_target.load_op == RafxLoadOp::Clear {
                    color_descriptor.set_clear_color(color_target.clear_value.into());
                }

                if let Some(resolve_target) = color_target.resolve_target {
                    color_descriptor.set_resolve_texture(Some(
                        resolve_target
                            .texture()
                            .metal_texture()
                            .unwrap()
                            .metal_texture(),
                    ));
                    color_descriptor
                        .set_resolve_level(color_target.resolve_mip_slice.unwrap_or(0) as _);
                    color_descriptor.set_resolve_slice(color_target.array_slice.unwrap_or(0) as _);
                }
            }

            if let Some(depth_target) = depth_target {
                let depth_descriptor = descriptor.depth_attachment().unwrap();
                let texture = depth_target
                    .render_target
                    .texture()
                    .metal_texture()
                    .unwrap();

                // Ensure current_render_targets_width/current_render_targets_depth are set
                extents = depth_target.render_target.texture().texture_def().extents;
                inner.current_render_targets_width = extents.width;
                inner.current_render_targets_height = extents.height;

                depth_descriptor.set_texture(Some(texture.metal_texture()));
                depth_descriptor.set_level(depth_target.mip_slice.unwrap_or(0) as _);
                depth_descriptor.set_slice(depth_target.array_slice.unwrap_or(0) as _);
                depth_descriptor.set_load_action(depth_target.depth_load_op.into());
                depth_descriptor.set_store_action(depth_target.depth_store_op.into());

                if depth_target.depth_load_op == RafxLoadOp::Clear {
                    depth_descriptor.set_clear_depth(depth_target.clear_value.depth as f64);
                }

                let has_stencil = texture.texture_def().format.has_stencil();
                if has_stencil {
                    let stencil_descriptor = descriptor.stencil_attachment().unwrap();
                    stencil_descriptor.set_texture(Some(texture.metal_texture()));
                    stencil_descriptor.set_level(depth_target.mip_slice.unwrap_or(0) as _);
                    stencil_descriptor.set_slice(depth_target.array_slice.unwrap_or(0) as _);
                    stencil_descriptor.set_load_action(depth_target.stencil_load_op.into());
                    stencil_descriptor.set_store_action(depth_target.stencil_store_op.into());
                } else {
                    //let stencil_descriptor = descriptor.stencil_attachment().unwrap();
                    //stencil_descriptor.set_load_action(RafxStoreOp::DontCare.into());
                    //stencil_descriptor.set_store_action(RafxStoreOp::DontCare.into());
                }
            } else {
                // let depth_descriptor = descriptor.depth_attachment().unwrap();
                // depth_descriptor.set_load_action(RafxStoreOp::DontCare.into());
                // depth_descriptor.set_store_action(RafxStoreOp::DontCare.into());
                // let stencil_descriptor = descriptor.stencil_attachment().unwrap();
                // stencil_descriptor.set_load_action(RafxStoreOp::DontCare.into());
                // stencil_descriptor.set_store_action(RafxStoreOp::DontCare.into());
            }

            // end encoders
            Self::do_end_current_encoders(&self.queue, &mut *inner, false)?;
            let cmd_buffer = inner.command_buffer.as_ref().unwrap();
            let render_encoder = cmd_buffer.new_render_command_encoder(descriptor);
            inner.render_encoder = Some(render_encoder.to_owned());
            self.wait_for_barriers(&*inner)?;
            // set heaps?

            Ok(())
        });
        result?;

        self.cmd_set_viewport(
            0.0,
            0.0,
            extents.width as f32,
            extents.height as f32,
            0.0,
            1.0,
        )?;

        self.cmd_set_scissor(0, 0, extents.width, extents.height)
    }

    pub fn end_current_encoders(
        &self,
        force_barrier: bool,
    ) -> RafxResult<()> {
        Self::do_end_current_encoders(&self.queue, &mut *self.inner.borrow_mut(), force_barrier)
    }

    pub fn do_end_current_encoders(
        queue: &RafxQueueMetal,
        inner: &mut RafxCommandBufferMetalInner,
        force_barrier: bool,
    ) -> RafxResult<()> {
        let barrier_flags = queue.barrier_flags();

        if let Some(render_encoder) = inner.render_encoder.take() {
            if !barrier_flags.is_empty() || force_barrier {
                render_encoder.update_fence(queue.metal_fence(), MTLRenderStages::Fragment);
                queue.add_barrier_flags(BarrierFlagsMetal::FENCE);
            }

            render_encoder.end_encoding();
        } else if let Some(compute_encoder) = inner.compute_encoder.take() {
            if !barrier_flags.is_empty() || force_barrier {
                compute_encoder.update_fence(queue.metal_fence());
                queue.add_barrier_flags(BarrierFlagsMetal::FENCE);
            }

            compute_encoder.end_encoding();
        } else if let Some(blit_encoder) = inner.blit_encoder.take() {
            if !barrier_flags.is_empty() || force_barrier {
                blit_encoder.update_fence(queue.metal_fence());
                queue.add_barrier_flags(BarrierFlagsMetal::FENCE);
            }

            blit_encoder.end_encoding();
        }

        Ok(())
    }

    fn wait_for_barriers(
        &self,
        inner: &RafxCommandBufferMetalInner,
    ) -> RafxResult<()> {
        let barrier_flags = self.queue.barrier_flags();
        if barrier_flags.is_empty() {
            return Ok(());
        }

        //TODO: Add support for memory barriers to metal_rs so this can be more fine-grained
        // (use memoryBarrierWithScope)

        if let Some(render_encoder) = &inner.render_encoder {
            render_encoder.wait_for_fence(self.queue.metal_fence(), MTLRenderStages::Vertex);
        } else if let Some(compute_encoder) = &inner.compute_encoder {
            compute_encoder.wait_for_fence(self.queue.metal_fence());
        } else if let Some(blit_encoder) = &inner.blit_encoder {
            blit_encoder.wait_for_fence(self.queue.metal_fence());
        }

        self.queue.clear_barrier_flags();
        Ok(())
    }

    pub fn cmd_unbind_render_targets(&self) -> RafxResult<()> {
        // no action necessary
        Ok(())
    }

    pub fn cmd_set_viewport(
        &self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        depth_min: f32,
        depth_max: f32,
    ) -> RafxResult<()> {
        self.inner
            .borrow()
            .render_encoder
            .as_ref()
            .unwrap()
            .set_viewport(MTLViewport {
                originX: x as _,
                originY: y as _,
                width: width as _,
                height: height as _,
                znear: depth_min as _,
                zfar: depth_max as _,
            });

        Ok(())
    }

    pub fn cmd_set_scissor(
        &self,
        mut x: u32,
        mut y: u32,
        mut width: u32,
        mut height: u32,
    ) -> RafxResult<()> {
        let inner = self.inner.borrow();
        let max_x = inner.current_render_targets_width;
        let max_y = inner.current_render_targets_height;

        x = x.min(max_x);
        y = y.min(max_y);
        width = width.min(max_x - x);
        height = height.min(max_y - y);

        inner
            .render_encoder
            .as_ref()
            .unwrap()
            .set_scissor_rect(MTLScissorRect {
                x: x as _,
                y: y as _,
                width: width as _,
                height: height as _,
            });

        Ok(())
    }

    pub fn cmd_set_stencil_reference_value(
        &self,
        value: u32,
    ) -> RafxResult<()> {
        self.inner
            .borrow()
            .render_encoder
            .as_ref()
            .unwrap()
            .set_stencil_reference_value(value);
        Ok(())
    }

    pub fn cmd_bind_pipeline(
        &self,
        pipeline: &RafxPipelineMetal,
    ) -> RafxResult<()> {
        objc::rc::autoreleasepool(|| {
            let mut inner = self.inner.borrow_mut();
            let last_pipeline_type = inner.last_pipeline_type;
            inner.last_pipeline_type = Some(pipeline.pipeline_type());

            let barrier_required = last_pipeline_type != Some(pipeline.pipeline_type());

            match pipeline.pipeline_type() {
                RafxPipelineType::Graphics => {
                    let render_encoder = inner.render_encoder.as_ref().unwrap();
                    let render_encoder_info = pipeline.render_encoder_info.as_ref().unwrap();
                    render_encoder
                        .set_render_pipeline_state(pipeline.metal_render_pipeline().unwrap());
                    render_encoder.set_cull_mode(render_encoder_info.mtl_cull_mode);
                    render_encoder
                        .set_front_facing_winding(render_encoder_info.mtl_front_facing_winding);
                    render_encoder
                        .set_triangle_fill_mode(render_encoder_info.mtl_triangle_fill_mode);
                    render_encoder.set_depth_bias(
                        render_encoder_info.mtl_depth_bias,
                        render_encoder_info.mtl_depth_bias_slope_scaled,
                        0.0,
                    );
                    render_encoder.set_depth_clip_mode(render_encoder_info.mtl_depth_clip_mode);
                    if let Some(mtl_depth_stencil_state) =
                        &render_encoder_info.mtl_depth_stencil_state
                    {
                        render_encoder.set_depth_stencil_state(mtl_depth_stencil_state);
                    }

                    inner.primitive_type = render_encoder_info.mtl_primitive_type;
                    self.flush_render_targets_to_make_readable(&mut *inner);
                }
                RafxPipelineType::Compute => {
                    if !inner.compute_encoder.is_some() {
                        Self::do_end_current_encoders(&self.queue, &mut *inner, barrier_required)?;

                        let compute_encoder = inner
                            .command_buffer
                            .as_ref()
                            .unwrap()
                            .new_compute_command_encoder();
                        inner.compute_encoder = Some(compute_encoder.to_owned());
                    }

                    let compute_encoder_info = pipeline.compute_encoder_info.as_ref().unwrap();
                    let compute_threads_per_group = compute_encoder_info.compute_threads_per_group;
                    inner.compute_threads_per_group_x = compute_threads_per_group[0];
                    inner.compute_threads_per_group_y = compute_threads_per_group[1];
                    inner.compute_threads_per_group_z = compute_threads_per_group[2];

                    inner
                        .compute_encoder
                        .as_ref()
                        .unwrap()
                        .set_compute_pipeline_state(pipeline.metal_compute_pipeline().unwrap());
                    self.flush_render_targets_to_make_readable(&mut *inner);
                }
            }

            Ok(())
        })
    }

    fn flush_render_targets_to_make_readable(
        &self,
        inner: &mut RafxCommandBufferMetalInner,
    ) {
        if let Some(render_encoder) = &inner.render_encoder {
            for attachment in &inner.render_targets_to_make_readable {
                render_encoder.use_resource(
                    attachment
                        .texture()
                        .metal_texture()
                        .unwrap()
                        .metal_texture(),
                    MTLResourceUsage::Read,
                );
            }

            inner.render_targets_to_make_readable.clear();
        } else if let Some(compute_encoder) = &inner.compute_encoder {
            for attachment in &inner.render_targets_to_make_readable {
                compute_encoder.use_resource(
                    attachment
                        .texture()
                        .metal_texture()
                        .unwrap()
                        .metal_texture(),
                    MTLResourceUsage::Read,
                );
            }

            inner.render_targets_to_make_readable.clear();
        }
    }

    pub fn cmd_bind_vertex_buffers(
        &self,
        first_binding: u32,
        bindings: &[RafxVertexBufferBinding],
    ) -> RafxResult<()> {
        let inner = self.inner.borrow();
        let render_encoder = inner.render_encoder.as_ref().unwrap();

        let mut binding_index = first_binding;
        for binding in bindings {
            render_encoder.set_vertex_buffer(
                super::util::vertex_buffer_adjusted_buffer_index(binding_index),
                Some(binding.buffer.metal_buffer().unwrap().metal_buffer()),
                binding.offset as _,
            );

            binding_index += 1;
        }

        Ok(())
    }

    pub fn cmd_bind_index_buffer(
        &self,
        binding: &RafxIndexBufferBinding,
    ) -> RafxResult<()> {
        let mut inner = self.inner.borrow_mut();
        inner.current_index_buffer = Some(
            binding
                .buffer
                .metal_buffer()
                .unwrap()
                .metal_buffer()
                .to_owned(),
        );
        inner.current_index_buffer_offset = binding.offset;
        inner.current_index_buffer_type = binding.index_type.into();
        inner.current_index_buffer_stride = match binding.index_type {
            RafxIndexType::Uint32 => std::mem::size_of::<u32>() as _,
            RafxIndexType::Uint16 => std::mem::size_of::<u16>() as _,
        };
        Ok(())
    }

    pub fn cmd_bind_descriptor_set(
        &self,
        descriptor_set_array: &RafxDescriptorSetArrayMetal,
        index: u32,
    ) -> RafxResult<()> {
        let (buffer, offset) = descriptor_set_array
            .metal_argument_buffer_and_offset(index)
            .unwrap();
        self.do_bind_descriptor_set(
            &*self.inner.borrow(),
            descriptor_set_array
                .root_signature()
                .metal_root_signature()
                .unwrap(),
            descriptor_set_array.set_index(),
            buffer,
            offset,
            index,
            descriptor_set_array.argument_buffer_data().unwrap(),
        )
    }

    pub fn cmd_bind_descriptor_set_handle(
        &self,
        root_signature: &RafxRootSignatureMetal,
        set_index: u32,
        descriptor_set_handle: &RafxDescriptorSetHandleMetal,
    ) -> RafxResult<()> {
        let buffer = descriptor_set_handle.metal_buffer();
        let offset = descriptor_set_handle.offset();
        let array_index = descriptor_set_handle.array_index();
        self.do_bind_descriptor_set(
            &*self.inner.borrow(),
            root_signature,
            set_index,
            buffer,
            offset,
            array_index,
            descriptor_set_handle.argument_buffer_data(),
        )
    }

    fn do_bind_descriptor_set(
        &self,
        inner: &RafxCommandBufferMetalInner,
        root_signature: &RafxRootSignatureMetal,
        set_index: u32,
        argument_buffer: &metal_rs::BufferRef,
        argument_buffer_offset: u32,
        array_index: u32,
        argument_buffer_data: &ArgumentBufferData,
    ) -> RafxResult<()> {
        match root_signature.pipeline_type() {
            RafxPipelineType::Graphics => {
                let render_encoder = inner
                    .render_encoder
                    .as_ref()
                    .ok_or("Must bind render targets before binding graphics descriptor sets")?;
                render_encoder.set_vertex_buffer(
                    set_index as _,
                    Some(argument_buffer),
                    argument_buffer_offset as _,
                );
                render_encoder.set_fragment_buffer(
                    set_index as _,
                    Some(argument_buffer),
                    argument_buffer_offset as _,
                );
                argument_buffer_data
                    .make_resources_resident_render_encoder(array_index, render_encoder);
            }
            RafxPipelineType::Compute => {
                let compute_encoder = inner
                    .compute_encoder
                    .as_ref()
                    .ok_or("Must bind compute pipeline before binding compute descriptor sets")?;
                compute_encoder.set_buffer(
                    set_index as _,
                    Some(argument_buffer),
                    argument_buffer_offset as _,
                );
                argument_buffer_data
                    .make_resources_resident_compute_encoder(array_index, compute_encoder);
            }
        }

        Ok(())
    }

    pub fn cmd_draw(
        &self,
        vertex_count: u32,
        first_vertex: u32,
    ) -> RafxResult<()> {
        let inner = self.inner.borrow();
        inner.render_encoder.as_ref().unwrap().draw_primitives(
            inner.primitive_type,
            first_vertex as _,
            vertex_count as _,
        );
        Ok(())
    }

    pub fn cmd_draw_instanced(
        &self,
        vertex_count: u32,
        first_vertex: u32,
        instance_count: u32,
        first_instance: u32,
    ) -> RafxResult<()> {
        let features = self.queue.device_context().metal_features();
        if !features.supports_base_vertex_instance_drawing {
            assert_eq!(first_instance, 0);
        }

        let inner = self.inner.borrow();

        if first_instance == 0 {
            inner
                .render_encoder
                .as_ref()
                .unwrap()
                .draw_primitives_instanced(
                    inner.primitive_type,
                    first_vertex as _,
                    vertex_count as _,
                    instance_count as _,
                );
        } else {
            inner
                .render_encoder
                .as_ref()
                .unwrap()
                .draw_primitives_instanced_base_instance(
                    inner.primitive_type,
                    first_vertex as _,
                    vertex_count as _,
                    instance_count as _,
                    first_instance as _,
                );
        }
        Ok(())
    }

    pub fn cmd_draw_indexed(
        &self,
        index_count: u32,
        first_index: u32,
        vertex_offset: i32,
    ) -> RafxResult<()> {
        let features = self.queue.device_context().metal_features();
        if !features.supports_base_vertex_instance_drawing {
            assert_eq!(vertex_offset, 0);
        }

        let inner = self.inner.borrow();
        let stride = inner.current_index_buffer_stride;
        if vertex_offset == 0 {
            inner
                .render_encoder
                .as_ref()
                .unwrap()
                .draw_indexed_primitives(
                    inner.primitive_type,
                    index_count as _,
                    inner.current_index_buffer_type,
                    inner.current_index_buffer.as_ref().unwrap(),
                    (stride * first_index) as _,
                );
        } else {
            inner
                .render_encoder
                .as_ref()
                .unwrap()
                .draw_indexed_primitives_instanced_base_instance(
                    inner.primitive_type,
                    index_count as _,
                    inner.current_index_buffer_type,
                    inner.current_index_buffer.as_ref().unwrap(),
                    (stride * first_index) as _,
                    1,
                    vertex_offset as _,
                    0,
                );
        }

        Ok(())
    }

    pub fn cmd_draw_indexed_instanced(
        &self,
        index_count: u32,
        first_index: u32,
        instance_count: u32,
        first_instance: u32,
        vertex_offset: i32,
    ) -> RafxResult<()> {
        let features = self.queue.device_context().metal_features();
        if !features.supports_base_vertex_instance_drawing {
            assert_eq!(vertex_offset, 0);
            assert_eq!(first_instance, 0);
        }

        let inner = self.inner.borrow();
        let stride = inner.current_index_buffer_stride;
        if vertex_offset == 0 && first_instance == 0 {
            inner
                .render_encoder
                .as_ref()
                .unwrap()
                .draw_indexed_primitives_instanced(
                    inner.primitive_type,
                    index_count as _,
                    inner.current_index_buffer_type,
                    inner.current_index_buffer.as_ref().unwrap(),
                    (stride * first_index) as _,
                    instance_count as _,
                );
        } else {
            inner
                .render_encoder
                .as_ref()
                .unwrap()
                .draw_indexed_primitives_instanced_base_instance(
                    inner.primitive_type,
                    index_count as _,
                    inner.current_index_buffer_type,
                    inner.current_index_buffer.as_ref().unwrap(),
                    (stride * first_index) as _,
                    instance_count as _,
                    vertex_offset as _,
                    first_instance as _,
                );
        }

        Ok(())
    }

    pub fn cmd_dispatch(
        &self,
        group_count_x: u32,
        group_count_y: u32,
        group_count_z: u32,
    ) -> RafxResult<()> {
        let inner = self.inner.borrow();
        self.wait_for_barriers(&*inner)?;
        let thread_per_group = MTLSize {
            width: inner.compute_threads_per_group_x as _,
            height: inner.compute_threads_per_group_y as _,
            depth: inner.compute_threads_per_group_z as _,
        };

        let group_count = MTLSize {
            width: group_count_x as _,
            height: group_count_y as _,
            depth: group_count_z as _,
        };

        inner
            .compute_encoder
            .as_ref()
            .unwrap()
            .dispatch_thread_groups(group_count, thread_per_group);
        Ok(())
    }

    pub fn cmd_resource_barrier(
        &self,
        buffer_barriers: &[RafxBufferBarrier],
        texture_barriers: &[RafxTextureBarrier],
        render_target_barriers: &[RafxRenderTargetBarrier],
    ) -> RafxResult<()> {
        if !buffer_barriers.is_empty() {
            self.queue.add_barrier_flags(BarrierFlagsMetal::BUFFERS);
        }

        if !texture_barriers.is_empty() {
            self.queue.add_barrier_flags(BarrierFlagsMetal::TEXTURES);
        }

        if !render_target_barriers.is_empty() {
            self.queue
                .add_barrier_flags(BarrierFlagsMetal::RENDER_TARGETS);

            let mut inner = self.inner.borrow_mut();
            for rt in render_target_barriers {
                if rt.src_state.intersects(RafxResourceState::RENDER_TARGET)
                    && rt.dst_state.intersects(
                        RafxResourceState::UNORDERED_ACCESS | RafxResourceState::SHADER_RESOURCE,
                    )
                {
                    inner
                        .render_targets_to_make_readable
                        .insert(rt.render_target.metal_render_target().unwrap().clone());
                }
            }
        }

        Ok(())
    }

    pub fn cmd_copy_buffer_to_buffer(
        &self,
        src_buffer: &RafxBufferMetal,
        dst_buffer: &RafxBufferMetal,
        src_offset: u64,
        dst_offset: u64,
        size: u64,
    ) -> RafxResult<()> {
        let mut inner = self.inner.borrow_mut();
        let blit_encoder = inner.blit_encoder.as_ref();
        let blit_encoder = match blit_encoder {
            Some(x) => x,
            None => {
                let result: RafxResult<&metal_rs::BlitCommandEncoderRef> =
                    objc::rc::autoreleasepool(|| {
                        Self::do_end_current_encoders(&self.queue, &mut *inner, false)?;
                        let encoder = inner
                            .command_buffer
                            .as_ref()
                            .unwrap()
                            .new_blit_command_encoder();
                        inner.blit_encoder = Some(encoder.to_owned());
                        Ok(inner.blit_encoder.as_ref().unwrap().as_ref())
                    });
                result?
            }
        };

        blit_encoder.copy_from_buffer(
            src_buffer.metal_buffer(),
            src_offset as _,
            dst_buffer.metal_buffer(),
            dst_offset as _,
            size as _,
        );
        Ok(())
    }

    pub fn cmd_copy_buffer_to_texture(
        &self,
        src_buffer: &RafxBufferMetal,
        dst_texture: &RafxTextureMetal,
        params: &RafxCmdCopyBufferToTextureParams,
    ) -> RafxResult<()> {
        let mut inner = self.inner.borrow_mut();
        let blit_encoder = inner.blit_encoder.as_ref();
        let blit_encoder = match blit_encoder {
            Some(x) => x,
            None => {
                let result: RafxResult<&metal_rs::BlitCommandEncoderRef> =
                    objc::rc::autoreleasepool(|| {
                        Self::do_end_current_encoders(&self.queue, &mut *inner, false)?;
                        let encoder = inner
                            .command_buffer
                            .as_ref()
                            .unwrap()
                            .new_blit_command_encoder();
                        inner.blit_encoder = Some(encoder.to_owned());
                        Ok(inner.blit_encoder.as_ref().unwrap().as_ref())
                    });
                result?
            }
        };

        let texture_def = dst_texture.texture_def();
        let width = 1.max(texture_def.extents.width >> params.mip_level);
        let height = 1.max(texture_def.extents.height >> params.mip_level);
        let depth = 1.max(texture_def.extents.depth >> params.mip_level);

        let format_size_in_bytes = dst_texture
            .texture_def()
            .format
            .size_of_format_in_bytes()
            .unwrap();

        let device_info = self.queue.device_context().device_info();
        let texture_alignment = device_info.upload_buffer_texture_alignment;
        let row_alignment = device_info.upload_buffer_texture_row_alignment;

        let source_bytes_per_row = rafx_base::memory::round_size_up_to_alignment_u32(
            width as u32 * format_size_in_bytes as u32,
            row_alignment,
        );
        let source_bytes_per_image = rafx_base::memory::round_size_up_to_alignment_u32(
            height * source_bytes_per_row,
            texture_alignment,
        );

        let source_size = MTLSize {
            width: width as _,
            height: height as _,
            depth: depth as _,
        };

        blit_encoder.copy_from_buffer_to_texture(
            src_buffer.metal_buffer(),
            params.buffer_offset as _,
            source_bytes_per_row as _,
            source_bytes_per_image as _,
            source_size,
            dst_texture.metal_texture(),
            params.array_layer as _,
            params.mip_level as _,
            MTLOrigin { x: 0, y: 0, z: 0 },
            MTLBlitOption::empty(),
        );
        Ok(())
    }

    pub fn cmd_blit(
        &self,
        _src_texture: &RafxTextureMetal,
        _dst_texture: &RafxTextureMetal,
        _params: &RafxCmdBlitParams,
    ) -> RafxResult<()> {
        //TODO: Implementing this requires having a custom shader/pipeline loaded and drawing
        //triangles
        unimplemented!();
    }
}
