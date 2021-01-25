use crate::vulkan::*;
use crate::*;
use ash::version::DeviceV1_0;
use ash::vk;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub struct RafxCommandBufferVulkan {
    device_context: RafxDeviceContextVulkan,
    vk_command_pool: vk::CommandPool,
    vk_command_buffer: vk::CommandBuffer,
    queue_type: RafxQueueType,
    queue_family_index: u32,
    has_active_renderpass: AtomicBool,
}

impl Into<RafxCommandBuffer> for RafxCommandBufferVulkan {
    fn into(self) -> RafxCommandBuffer {
        RafxCommandBuffer::Vk(self)
    }
}

impl RafxCommandBufferVulkan {
    pub fn new(
        command_pool: &RafxCommandPoolVulkan,
        command_buffer_def: &RafxCommandBufferDef,
    ) -> RafxResult<RafxCommandBufferVulkan> {
        log::trace!(
            "Creating command buffers from pool {:?}",
            command_pool.vk_command_pool()
        );
        let command_buffer_level = if command_buffer_def.is_secondary {
            vk::CommandBufferLevel::SECONDARY
        } else {
            vk::CommandBufferLevel::PRIMARY
        };

        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool.vk_command_pool())
            .level(command_buffer_level)
            .command_buffer_count(1);

        let vk_command_buffer = unsafe {
            command_pool
                .device_context()
                .device()
                .allocate_command_buffers(&command_buffer_allocate_info)
        }?[0];

        Ok(RafxCommandBufferVulkan {
            device_context: command_pool.device_context().clone(),
            vk_command_pool: command_pool.vk_command_pool(),
            vk_command_buffer,
            queue_type: command_pool.queue_type(),
            queue_family_index: command_pool.queue_family_index(),
            has_active_renderpass: AtomicBool::new(false),
        })
    }

    pub fn vk_command_buffer(&self) -> vk::CommandBuffer {
        self.vk_command_buffer
    }

    pub fn begin(&self) -> RafxResult<()> {
        //TODO: Use one-time-submit?
        let command_buffer_usage_flags = vk::CommandBufferUsageFlags::empty();

        let begin_info = vk::CommandBufferBeginInfo::builder().flags(command_buffer_usage_flags);

        unsafe {
            self.device_context
                .device()
                .begin_command_buffer(self.vk_command_buffer, &*begin_info)?;
        }

        Ok(())
    }

    pub fn end(&self) -> RafxResult<()> {
        if self.has_active_renderpass.load(Ordering::Relaxed) {
            unsafe {
                self.device_context
                    .device()
                    .cmd_end_render_pass(self.vk_command_buffer);
            }

            self.has_active_renderpass.store(false, Ordering::Relaxed);
        }

        unsafe {
            self.device_context
                .device()
                .end_command_buffer(self.vk_command_buffer)?;
        }

        Ok(())
    }

    pub fn return_to_pool(&self) -> RafxResult<()> {
        unsafe {
            self.device_context
                .device()
                .free_command_buffers(self.vk_command_pool, &[self.vk_command_buffer]);
        }

        Ok(())
    }

    pub fn cmd_bind_render_targets(
        &self,
        color_targets: &[RafxColorRenderTargetBinding],
        depth_target: Option<RafxDepthRenderTargetBinding>,
    ) -> RafxResult<()> {
        if self.has_active_renderpass.load(Ordering::Relaxed) {
            self.cmd_unbind_render_targets()?;
        }

        if color_targets.is_empty() && depth_target.is_none() {
            Err("No color or depth target supplied to cmd_bind_render_targets")?;
        }

        let (renderpass, framebuffer) = {
            let resource_cache = self.device_context.resource_cache();
            let mut resource_cache = resource_cache.inner.lock().unwrap();

            let renderpass = resource_cache.renderpass_cache.get_or_create_renderpass(
                &self.device_context,
                color_targets,
                depth_target.as_ref(),
            )?;
            let framebuffer = resource_cache.framebuffer_cache.get_or_create_framebuffer(
                &self.device_context,
                &renderpass,
                color_targets,
                depth_target.as_ref(),
            )?;

            (renderpass, framebuffer)
        };

        let barriers = {
            let mut barriers = Vec::with_capacity(color_targets.len() + 1);
            for color_target in color_targets {
                if color_target
                    .render_target
                    .vk_render_target()
                    .unwrap()
                    .take_is_undefined_layout()
                {
                    log::trace!(
                        "Transition RT {:?} from {:?} to {:?}",
                        color_target,
                        RafxResourceState::UNDEFINED,
                        RafxResourceState::RENDER_TARGET
                    );
                    barriers.push(RafxRenderTargetBarrier::state_transition(
                        &color_target.render_target,
                        RafxResourceState::UNDEFINED,
                        RafxResourceState::RENDER_TARGET,
                    ));
                }
            }

            if let Some(depth_target) = &depth_target {
                if depth_target
                    .render_target
                    .vk_render_target()
                    .unwrap()
                    .take_is_undefined_layout()
                {
                    log::trace!(
                        "Transition RT {:?} from {:?} to {:?}",
                        depth_target,
                        RafxResourceState::UNDEFINED,
                        RafxResourceState::DEPTH_WRITE
                    );
                    barriers.push(RafxRenderTargetBarrier::state_transition(
                        &depth_target.render_target,
                        RafxResourceState::UNDEFINED,
                        RafxResourceState::DEPTH_WRITE,
                    ));
                }
            }

            barriers
        };

        let render_area = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D {
                width: framebuffer.width(),
                height: framebuffer.height(),
            },
        };

        let mut clear_values = Vec::with_capacity(color_targets.len() + 1);
        let mut has_resolve_target = false;
        for color_target in color_targets {
            clear_values.push(color_target.clear_value.clone().into());
            if color_target.resolve_target.is_some() {
                has_resolve_target = true;
            }
        }

        // If we resolve, then there will be images in the framebuffer. The clear color array must
        // be equal-sized to the framebuffer images array.
        if has_resolve_target {
            for _ in color_targets {
                // Actual value doesn't matter, this is for a resolve target with DONT_CARE load op
                clear_values.push(clear_values[0]);
            }
        }

        if let Some(depth_target) = &depth_target {
            clear_values.push(depth_target.clear_value.clone().into());
        }

        if !barriers.is_empty() {
            self.cmd_resource_barrier(&[], &[], &barriers)?;
        }

        let begin_renderpass_create_info = vk::RenderPassBeginInfo::builder()
            .render_pass(renderpass.vk_renderpass())
            .framebuffer(framebuffer.vk_framebuffer())
            .render_area(render_area)
            .clear_values(&clear_values);

        unsafe {
            self.device_context.device().cmd_begin_render_pass(
                self.vk_command_buffer,
                &*begin_renderpass_create_info,
                vk::SubpassContents::INLINE,
            );
        }

        self.has_active_renderpass.store(true, Ordering::Relaxed);

        self.cmd_set_viewport(
            0.0,
            0.0,
            framebuffer.width() as f32,
            framebuffer.height() as f32,
            0.0,
            1.0,
        )
        .unwrap();
        self.cmd_set_scissor(0, 0, framebuffer.width(), framebuffer.height())
            .unwrap();

        Ok(())
    }

    pub fn cmd_unbind_render_targets(&self) -> RafxResult<()> {
        unsafe {
            self.device_context
                .device()
                .cmd_end_render_pass(self.vk_command_buffer);
            self.has_active_renderpass.store(false, Ordering::Relaxed);
        }

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
        unsafe {
            // We invert the viewport by using negative height and setting y = y + height
            // This is supported in vulkan 1.1 or 1.0 with an extension
            self.device_context.device().cmd_set_viewport(
                self.vk_command_buffer,
                0,
                &[vk::Viewport {
                    x,
                    y: y + height,
                    width,
                    height: height * -1.0,
                    min_depth: depth_min,
                    max_depth: depth_max,
                }],
            );
        }
        Ok(())
    }

    pub fn cmd_set_scissor(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> RafxResult<()> {
        unsafe {
            self.device_context.device().cmd_set_scissor(
                self.vk_command_buffer,
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D {
                        x: x as i32,
                        y: y as i32,
                    },
                    extent: vk::Extent2D { width, height },
                }],
            );
        }
        Ok(())
    }

    pub fn cmd_set_stencil_reference_value(
        &self,
        value: u32,
    ) -> RafxResult<()> {
        unsafe {
            self.device_context.device().cmd_set_stencil_reference(
                self.vk_command_buffer,
                vk::StencilFaceFlags::FRONT_AND_BACK,
                value,
            );
        }
        Ok(())
    }

    pub fn cmd_bind_pipeline(
        &self,
        pipeline: &RafxPipelineVulkan,
    ) -> RafxResult<()> {
        //TODO: Add verification that the pipeline is compatible with the renderpass created by the targets
        let pipeline_bind_point =
            super::util::pipeline_type_pipeline_bind_point(pipeline.pipeline_type());

        unsafe {
            self.device_context.device().cmd_bind_pipeline(
                self.vk_command_buffer,
                pipeline_bind_point,
                pipeline.vk_pipeline(),
            );
        }
        Ok(())
    }

    pub fn cmd_bind_vertex_buffers(
        &self,
        first_binding: u32,
        bindings: &[RafxVertexBufferBinding],
    ) -> RafxResult<()> {
        let mut buffers = Vec::with_capacity(bindings.len());
        let mut offsets = Vec::with_capacity(bindings.len());
        for binding in bindings {
            buffers.push(binding.buffer.vk_buffer().unwrap().vk_buffer());
            offsets.push(binding.offset);
        }

        unsafe {
            self.device_context.device().cmd_bind_vertex_buffers(
                self.vk_command_buffer,
                first_binding,
                &buffers,
                &offsets,
            )
        }

        Ok(())
    }

    pub fn cmd_bind_index_buffer(
        &self,
        binding: &RafxIndexBufferBinding,
    ) -> RafxResult<()> {
        unsafe {
            self.device_context.device().cmd_bind_index_buffer(
                self.vk_command_buffer,
                binding.buffer.vk_buffer().unwrap().vk_buffer(),
                binding.offset,
                binding.index_type.into(),
            )
        }

        Ok(())
    }

    pub fn cmd_bind_descriptor_set(
        &self,
        descriptor_set_array: &RafxDescriptorSetArrayVulkan,
        index: u32,
    ) -> RafxResult<()> {
        self.cmd_bind_descriptor_set_handle(
            descriptor_set_array
                .root_signature()
                .vk_root_signature()
                .unwrap(),
            descriptor_set_array.set_index(),
            &descriptor_set_array.handle(index).unwrap(),
        )
    }

    pub fn cmd_bind_descriptor_set_handle(
        &self,
        root_signature: &RafxRootSignatureVulkan,
        set_index: u32,
        descriptor_set_handle: &RafxDescriptorSetHandleVulkan,
    ) -> RafxResult<()> {
        let bind_point = root_signature.pipeline_type();

        unsafe {
            self.device_context.device().cmd_bind_descriptor_sets(
                self.vk_command_buffer,
                super::util::pipeline_type_pipeline_bind_point(bind_point),
                root_signature.vk_pipeline_layout(),
                set_index,
                &[descriptor_set_handle.0],
                &[],
            )
        }

        Ok(())
    }

    pub fn cmd_draw(
        &self,
        vertex_count: u32,
        first_vertex: u32,
    ) -> RafxResult<()> {
        unsafe {
            self.device_context.device().cmd_draw(
                self.vk_command_buffer,
                vertex_count,
                1,
                first_vertex,
                0,
            )
        }

        Ok(())
    }

    pub fn cmd_draw_instanced(
        &self,
        vertex_count: u32,
        first_vertex: u32,
        instance_count: u32,
        first_instance: u32,
    ) -> RafxResult<()> {
        unsafe {
            self.device_context.device().cmd_draw(
                self.vk_command_buffer,
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
            )
        }

        Ok(())
    }

    pub fn cmd_draw_indexed(
        &self,
        index_count: u32,
        first_index: u32,
        vertex_offset: i32,
    ) -> RafxResult<()> {
        unsafe {
            self.device_context.device().cmd_draw_indexed(
                self.vk_command_buffer,
                index_count,
                1,
                first_index,
                vertex_offset,
                0,
            )
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
        unsafe {
            self.device_context.device().cmd_draw_indexed(
                self.vk_command_buffer,
                index_count,
                instance_count,
                first_index,
                vertex_offset,
                first_instance,
            )
        }

        Ok(())
    }

    pub fn cmd_dispatch(
        &self,
        group_count_x: u32,
        group_count_y: u32,
        group_count_z: u32,
    ) -> RafxResult<()> {
        unsafe {
            self.device_context.device().cmd_dispatch(
                self.vk_command_buffer,
                group_count_x,
                group_count_y,
                group_count_z,
            )
        }

        Ok(())
    }

    pub fn cmd_resource_barrier(
        &self,
        buffer_barriers: &[RafxBufferBarrier],
        texture_barriers: &[RafxTextureBarrier],
        render_target_barriers: &[RafxRenderTargetBarrier],
    ) -> RafxResult<()> {
        assert!(
            !self.has_active_renderpass.load(Ordering::Relaxed),
            "cmd_resource_barrier may not be called if render targets are bound"
        );

        let mut vk_image_barriers =
            Vec::with_capacity(texture_barriers.len() + render_target_barriers.len());
        let mut vk_buffer_barriers = Vec::with_capacity(buffer_barriers.len());

        let mut src_access_flags = vk::AccessFlags::empty();
        let mut dst_access_flags = vk::AccessFlags::empty();

        for barrier in buffer_barriers {
            let buffer = barrier.buffer.vk_buffer().unwrap();

            let mut vk_buffer_barrier = vk::BufferMemoryBarrier::builder()
                .src_access_mask(super::util::resource_state_to_access_flags(
                    barrier.src_state,
                ))
                .dst_access_mask(super::util::resource_state_to_access_flags(
                    barrier.dst_state,
                ))
                .buffer(buffer.vk_buffer())
                .size(vk::WHOLE_SIZE)
                .offset(0)
                .build();

            match &barrier.queue_transition {
                RafxBarrierQueueTransition::ReleaseTo(dst_queue_type) => {
                    vk_buffer_barrier.src_queue_family_index = self.queue_family_index;
                    vk_buffer_barrier.dst_queue_family_index =
                        super::util::queue_type_to_family_index(
                            &self.device_context,
                            *dst_queue_type,
                        );
                }
                RafxBarrierQueueTransition::AcquireFrom(src_queue_type) => {
                    vk_buffer_barrier.src_queue_family_index =
                        super::util::queue_type_to_family_index(
                            &self.device_context,
                            *src_queue_type,
                        );
                    vk_buffer_barrier.dst_queue_family_index = self.queue_family_index;
                }
                RafxBarrierQueueTransition::None => {
                    vk_buffer_barrier.src_queue_family_index = vk::QUEUE_FAMILY_IGNORED;
                    vk_buffer_barrier.dst_queue_family_index = vk::QUEUE_FAMILY_IGNORED;
                }
            }

            src_access_flags |= vk_buffer_barrier.src_access_mask;
            dst_access_flags |= vk_buffer_barrier.dst_access_mask;

            vk_buffer_barriers.push(vk_buffer_barrier);
        }

        fn image_subresource_range(
            aspect_mask: vk::ImageAspectFlags,
            array_slice: Option<u16>,
            mip_slice: Option<u8>,
        ) -> vk::ImageSubresourceRange {
            let mut subresource_range = vk::ImageSubresourceRange::builder()
                .aspect_mask(aspect_mask)
                .build();

            if let Some(array_slice) = array_slice {
                subresource_range.layer_count = 1;
                subresource_range.base_array_layer = array_slice as u32;
            } else {
                subresource_range.layer_count = vk::REMAINING_ARRAY_LAYERS;
                subresource_range.base_array_layer = 0;
            };

            if let Some(mip_slice) = mip_slice {
                subresource_range.level_count = 1;
                subresource_range.base_mip_level = mip_slice as u32;
            } else {
                subresource_range.level_count = vk::REMAINING_MIP_LEVELS;
                subresource_range.base_mip_level = 0;
            }

            subresource_range
        }

        fn set_queue_family_indices(
            vk_image_barrier: &mut vk::ImageMemoryBarrier,
            device_context: &RafxDeviceContextVulkan,
            self_queue_family_index: u32,
            queue_transition: &RafxBarrierQueueTransition,
        ) {
            match queue_transition {
                RafxBarrierQueueTransition::ReleaseTo(dst_queue_type) => {
                    vk_image_barrier.src_queue_family_index = self_queue_family_index;
                    vk_image_barrier.dst_queue_family_index =
                        super::util::queue_type_to_family_index(device_context, *dst_queue_type);
                }
                RafxBarrierQueueTransition::AcquireFrom(src_queue_type) => {
                    vk_image_barrier.src_queue_family_index =
                        super::util::queue_type_to_family_index(device_context, *src_queue_type);
                    vk_image_barrier.dst_queue_family_index = self_queue_family_index;
                }
                RafxBarrierQueueTransition::None => {
                    vk_image_barrier.src_queue_family_index = vk::QUEUE_FAMILY_IGNORED;
                    vk_image_barrier.dst_queue_family_index = vk::QUEUE_FAMILY_IGNORED;
                }
            }
        }

        for barrier in texture_barriers {
            let texture = barrier.texture.vk_texture().unwrap();

            let subresource_range = image_subresource_range(
                texture.vk_aspect_mask(),
                barrier.array_slice,
                barrier.mip_slice,
            );

            let old_layout =
                super::util::resource_state_to_image_layout(barrier.src_state).unwrap();
            let new_layout =
                super::util::resource_state_to_image_layout(barrier.dst_state).unwrap();
            log::trace!(
                "Transition texture {:?} from {:?} to {:?}",
                texture,
                old_layout,
                new_layout
            );

            let mut vk_image_barrier = vk::ImageMemoryBarrier::builder()
                .src_access_mask(super::util::resource_state_to_access_flags(
                    barrier.src_state,
                ))
                .dst_access_mask(super::util::resource_state_to_access_flags(
                    barrier.dst_state,
                ))
                .old_layout(old_layout)
                .new_layout(new_layout)
                .image(texture.vk_image())
                .subresource_range(subresource_range)
                .build();

            set_queue_family_indices(
                &mut vk_image_barrier,
                &self.device_context,
                self.queue_family_index,
                &barrier.queue_transition,
            );

            src_access_flags |= vk_image_barrier.src_access_mask;
            dst_access_flags |= vk_image_barrier.dst_access_mask;

            vk_image_barriers.push(vk_image_barrier);
        }

        for barrier in render_target_barriers {
            let render_target = barrier.render_target.vk_render_target().unwrap();

            let subresource_range = image_subresource_range(
                render_target.vk_aspect_mask(),
                barrier.array_slice,
                barrier.mip_slice,
            );

            let old_layout = if barrier
                .render_target
                .vk_render_target()
                .unwrap()
                .take_is_undefined_layout()
            {
                vk::ImageLayout::UNDEFINED
            } else {
                super::util::resource_state_to_image_layout(barrier.src_state).unwrap()
            };

            let new_layout =
                super::util::resource_state_to_image_layout(barrier.dst_state).unwrap();
            log::trace!(
                "Transition RT {:?} from {:?} to {:?}",
                render_target,
                old_layout,
                new_layout
            );

            let mut vk_image_barrier = vk::ImageMemoryBarrier::builder()
                .src_access_mask(super::util::resource_state_to_access_flags(
                    barrier.src_state,
                ))
                .dst_access_mask(super::util::resource_state_to_access_flags(
                    barrier.dst_state,
                ))
                .old_layout(old_layout)
                .new_layout(new_layout)
                .image(render_target.vk_image())
                .subresource_range(subresource_range)
                .build();

            set_queue_family_indices(
                &mut vk_image_barrier,
                &self.device_context,
                self.queue_family_index,
                &barrier.queue_transition,
            );

            src_access_flags |= vk_image_barrier.src_access_mask;
            dst_access_flags |= vk_image_barrier.dst_access_mask;

            vk_image_barriers.push(vk_image_barrier);
        }

        let src_stage_mask =
            super::util::determine_pipeline_stage_flags(self.queue_type, src_access_flags);
        let dst_stage_mask =
            super::util::determine_pipeline_stage_flags(self.queue_type, dst_access_flags);

        if !vk_buffer_barriers.is_empty() || !vk_image_barriers.is_empty() {
            unsafe {
                self.device_context.device().cmd_pipeline_barrier(
                    self.vk_command_buffer,
                    src_stage_mask,
                    dst_stage_mask,
                    vk::DependencyFlags::empty(),
                    &[],
                    &vk_buffer_barriers,
                    &vk_image_barriers,
                );
            }
        }

        Ok(())
    }

    pub fn cmd_copy_buffer_to_buffer(
        &self,
        src_buffer: &RafxBufferVulkan,
        dst_buffer: &RafxBufferVulkan,
        src_offset: u64,
        dst_offset: u64,
        size: u64,
    ) -> RafxResult<()> {
        unsafe {
            self.device_context.device().cmd_copy_buffer(
                self.vk_command_buffer,
                src_buffer.vk_buffer(),
                dst_buffer.vk_buffer(),
                &[vk::BufferCopy {
                    size,
                    src_offset,
                    dst_offset,
                }],
            );
        }

        Ok(())
    }

    pub fn cmd_copy_buffer_to_texture(
        &self,
        src_buffer: &RafxBufferVulkan,
        dst_texture: &RafxTextureVulkan,
        params: &RafxCmdCopyBufferToTextureParams,
    ) -> RafxResult<()> {
        let texture_def = dst_texture.texture_def();

        let width = 1.max(texture_def.extents.width >> params.mip_level);
        let height = 1.max(texture_def.extents.height >> params.mip_level);
        let depth = 1.max(texture_def.extents.depth >> params.mip_level);

        unsafe {
            self.device_context.device().cmd_copy_buffer_to_image(
                self.vk_command_buffer,
                src_buffer.vk_buffer(),
                dst_texture.vk_image(),
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[vk::BufferImageCopy {
                    image_extent: vk::Extent3D {
                        width,
                        height,
                        depth,
                    },
                    image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
                    image_subresource: vk::ImageSubresourceLayers {
                        aspect_mask: dst_texture.vk_aspect_mask(),
                        mip_level: params.mip_level as u32,
                        base_array_layer: params.array_layer as u32,
                        layer_count: 1,
                    },
                    buffer_offset: params.buffer_offset,
                    buffer_image_height: 0,
                    buffer_row_length: 0,
                }],
            );
        }

        Ok(())
    }

    pub fn cmd_blit(
        &self,
        src_texture: &RafxTextureVulkan,
        dst_texture: &RafxTextureVulkan,
        params: &RafxCmdBlitParams,
    ) -> RafxResult<()> {
        let src_aspect_mask =
            super::util::image_format_to_aspect_mask(src_texture.texture_def().format);
        let dst_aspect_mask =
            super::util::image_format_to_aspect_mask(dst_texture.texture_def().format);

        let mut src_subresource = vk::ImageSubresourceLayers::builder()
            .aspect_mask(src_aspect_mask)
            .mip_level(params.src_mip_level as u32)
            .build();
        let mut dst_subresource = vk::ImageSubresourceLayers::builder()
            .aspect_mask(dst_aspect_mask)
            .mip_level(params.dst_mip_level as u32)
            .build();

        if let Some(array_slices) = params.array_slices {
            src_subresource.base_array_layer = array_slices[0] as u32;
            dst_subresource.base_array_layer = array_slices[1] as u32;
            src_subresource.layer_count = 1;
            dst_subresource.layer_count = 1;
        } else {
            src_subresource.base_array_layer = 0;
            dst_subresource.base_array_layer = 0;
            src_subresource.layer_count = vk::REMAINING_ARRAY_LAYERS;
            dst_subresource.layer_count = vk::REMAINING_ARRAY_LAYERS;
        }

        let src_offsets = [
            vk::Offset3D {
                x: params.src_extents[0].width as i32,
                y: params.src_extents[0].height as i32,
                z: params.src_extents[0].depth as i32,
            },
            vk::Offset3D {
                x: params.src_extents[1].width as i32,
                y: params.src_extents[1].height as i32,
                z: params.src_extents[1].depth as i32,
            },
        ];

        let dst_offsets = [
            vk::Offset3D {
                x: params.dst_extents[0].width as i32,
                y: params.dst_extents[0].height as i32,
                z: params.dst_extents[0].depth as i32,
            },
            vk::Offset3D {
                x: params.dst_extents[1].width as i32,
                y: params.dst_extents[1].height as i32,
                z: params.dst_extents[1].depth as i32,
            },
        ];

        let image_blit = vk::ImageBlit::builder()
            .src_offsets(src_offsets)
            .src_subresource(src_subresource)
            .dst_offsets(dst_offsets)
            .dst_subresource(dst_subresource);

        unsafe {
            self.device_context.device().cmd_blit_image(
                self.vk_command_buffer,
                src_texture.vk_image(),
                super::util::resource_state_to_image_layout(params.src_state).unwrap(),
                dst_texture.vk_image(),
                super::util::resource_state_to_image_layout(params.dst_state).unwrap(),
                &[*image_blit],
                vk::Filter::LINEAR,
            );
        }

        Ok(())
    }
}
