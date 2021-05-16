use crate::gles3::{
    AttributeEnabledBits, BoundDescriptorSet, BoundVertexBuffer, CommandPoolGles3State,
    CommandPoolGles3StateInner, DescriptorSetArrayData, GlContext, Gles3PipelineInfo,
    RafxBufferGles3, RafxCommandPoolGles3, RafxDescriptorSetArrayGles3,
    RafxDescriptorSetHandleGles3, RafxPipelineGles3, RafxQueueGles3, RafxRootSignatureGles3,
    RafxTextureGles3, NONE_BUFFER, NONE_FRAMEBUFFER, NONE_PROGRAM, NONE_TEXTURE,
};
use crate::{
    RafxBufferBarrier, RafxCmdCopyBufferToTextureParams, RafxColorFlags,
    RafxColorRenderTargetBinding, RafxCommandBufferDef, RafxDepthStencilRenderTargetBinding,
    RafxExtents3D, RafxIndexBufferBinding, RafxIndexType, RafxLoadOp, RafxResourceType, RafxResult,
    RafxTextureBarrier, RafxVertexBufferBinding, MAX_DESCRIPTOR_SET_LAYOUTS,
};

use rafx_base::trust_cell::TrustCell;

use crate::gles3::conversions::{array_layer_to_cube_map_target, Gles3DepthStencilState};
use crate::gles3::gles3_bindings;

use crate::backends::gles3::{RafxRawImageGles3, RafxSamplerIndexGles3};
use crate::gles3::gl_type_util;
use crate::gles3::gles3_bindings::types::GLenum;
use std::sync::Arc;

#[derive(Debug)]
pub struct RafxCommandBufferGles3 {
    queue: RafxQueueGles3,
    command_pool_state: CommandPoolGles3State,
}

impl RafxCommandBufferGles3 {
    pub(crate) fn queue(&self) -> &RafxQueueGles3 {
        &self.queue
    }

    pub fn new(
        command_pool: &RafxCommandPoolGles3,
        _command_buffer_def: &RafxCommandBufferDef,
    ) -> RafxResult<RafxCommandBufferGles3> {
        Ok(RafxCommandBufferGles3 {
            queue: command_pool.queue().clone(),
            command_pool_state: command_pool.command_pool_state().clone(),
        })
    }

    pub fn begin(&self) -> RafxResult<()> {
        let mut state = self.command_pool_state.borrow_mut();
        assert!(!state.is_started);
        state.is_started = true;

        Ok(())
    }

    pub fn end(&self) -> RafxResult<()> {
        let mut state = self.command_pool_state.borrow_mut();
        assert!(state.is_started);

        let gl_context = self.queue.device_context().gl_context();

        // We purposely do not clear framebuffer_color_bound, framebuffer_depth_bound, or
        // framebuffer_stencil_bound. The framebuffer and tracking if a texture is bound should
        // be persisted across frames. The state is private to the command pool.

        state.is_started = false;
        assert!(state.surface_size.is_none());
        state.current_gl_pipeline_info = None;
        state.stencil_reference_value = 0;
        state.clear_bindings();
        Self::update_vertex_attributes_in_use(gl_context, &mut *state, 0)?;
        assert_eq!(state.vertex_attribute_enabled_bits, 0);
        for attribute in &mut state.vertex_attributes {
            *attribute = None;
        }
        for vertex_offset in &mut state.currently_bound_vertex_offset {
            *vertex_offset = None;
        }
        for bound_vertex_buffer in &mut state.bound_vertex_buffers {
            *bound_vertex_buffer = None;
        }
        state.index_buffer_byte_offset = 0;

        Ok(())
    }

    pub fn return_to_pool(&self) -> RafxResult<()> {
        // don't need to do anything
        Ok(())
    }

    fn bind_framebuffer(
        gl_context: &GlContext,
        texture: &RafxTextureGles3,
        attachment: GLenum,
        array_slice: u16,
        mip_slice: u8,
    ) -> RafxResult<()> {
        match texture.gl_raw_image() {
            // RafxRawImageGl::Renderbuffer(id) => {
            //     assert!(mip_slice.is_none());
            //     gl_context.gl_bind_renderbuffer(gles30::RENDERBUFFER, *id)?;
            //     if *id != NONE_RENDERBUFFER {
            //         gl_context.gl_framebuffer_renderbuffer(
            //             gles30::FRAMEBUFFER,
            //             attachment,
            //             gles30::RENDERBUFFER,
            //             *id,
            //         )?;
            //     }
            //
            //     gl_context.gl_bind_renderbuffer(gles30::RENDERBUFFER, NONE_RENDERBUFFER)?;
            // }
            RafxRawImageGles3::Texture(id) => {
                //TODO: texture array, cubemaps, mip levels. Requires ES 3.0, glFramebufferTextureLayer
                let target = texture.gl_target();
                gl_context.gl_bind_texture(target, *id)?;

                let mut subtarget = target;
                if subtarget == gles3_bindings::TEXTURE_CUBE_MAP {
                    subtarget = array_layer_to_cube_map_target(array_slice);
                }

                gl_context.gl_framebuffer_texture(
                    gles3_bindings::FRAMEBUFFER,
                    attachment,
                    subtarget,
                    *id,
                    mip_slice,
                )?;
                gl_context.gl_bind_texture(target, NONE_TEXTURE)?;
            }
        }

        Ok(())
    }

    fn unbind_framebuffer(
        gl_context: &GlContext,
        attachment: GLenum,
    ) -> RafxResult<()> {
        gl_context.gl_framebuffer_texture(
            gles3_bindings::FRAMEBUFFER,
            attachment,
            gles3_bindings::TEXTURE_2D,
            NONE_TEXTURE,
            0,
        )
    }

    pub fn cmd_begin_render_pass(
        &self,
        color_targets: &[RafxColorRenderTargetBinding],
        depth_target: Option<RafxDepthStencilRenderTargetBinding>,
    ) -> RafxResult<()> {
        let mut state = self.command_pool_state.borrow_mut();
        assert!(state.is_started);

        //TODO: Handle array slice/mip level
        //TODO: MSAA/resolving
        //TODO: glInvalidateFramebuffer (ES3 only)
        //TODO: Cache FBOs instead of re-create per frame
        if color_targets.is_empty() && depth_target.is_none() {
            Err("No color or depth target supplied to cmd_begin_render_pass")?;
        }

        let gl_context = self.queue.device_context().gl_context();
        let mut clear_mask = 0;
        let mut extents = RafxExtents3D::default();

        gl_context.gl_use_program(NONE_PROGRAM)?;
        gl_context.gl_bind_framebuffer(gles3_bindings::FRAMEBUFFER, state.framebuffer_id)?;

        for (index, render_target) in color_targets.iter().enumerate() {
            extents = render_target.texture.texture_def().extents;

            let gl_texture = render_target.texture.gles3_texture().unwrap();
            let attachment = gles3_bindings::COLOR_ATTACHMENT0 + index as u32;
            Self::bind_framebuffer(
                gl_context,
                gl_texture,
                attachment,
                render_target.array_slice.unwrap_or(0),
                render_target.mip_slice.unwrap_or(0),
            )?;

            if render_target.load_op == RafxLoadOp::Clear {
                let c = &render_target.clear_value.0;
                gl_context.gl_clear_color(c[0], c[1], c[2], c[3])?;
                clear_mask |= gles3_bindings::COLOR_BUFFER_BIT;
            }
        }

        for (i, is_bound) in state.framebuffer_color_bound.iter_mut().enumerate() {
            if i < color_targets.len() {
                *is_bound = true;
            } else {
                if *is_bound {
                    Self::unbind_framebuffer(
                        gl_context,
                        gles3_bindings::COLOR_ATTACHMENT0 + i as u32,
                    )?;
                    *is_bound = false;
                }
            }
        }

        let mut has_depth = false;
        let mut has_stencil = false;
        if let Some(depth_target) = depth_target {
            let format = depth_target.texture.texture_def().format;
            if format.has_depth() {
                extents = depth_target.texture.texture_def().extents;

                let gl_texture = depth_target.texture.gles3_texture().unwrap();
                let attachment = gles3_bindings::DEPTH_ATTACHMENT;
                Self::bind_framebuffer(
                    gl_context,
                    gl_texture,
                    attachment,
                    depth_target.array_slice.unwrap_or(0),
                    depth_target.mip_slice.unwrap_or(0),
                )?;

                if depth_target.depth_load_op == RafxLoadOp::Clear {
                    gl_context.gl_clear_depthf(depth_target.clear_value.depth)?;
                    clear_mask |= gles3_bindings::DEPTH_BUFFER_BIT;
                }

                has_depth = true;
            }

            if format.has_stencil() {
                extents = depth_target.texture.texture_def().extents;

                let gl_texture = depth_target.texture.gles3_texture().unwrap();
                let attachment = gles3_bindings::STENCIL_ATTACHMENT;
                Self::bind_framebuffer(
                    gl_context,
                    gl_texture,
                    attachment,
                    depth_target.array_slice.unwrap_or(0),
                    depth_target.mip_slice.unwrap_or(0),
                )?;

                if depth_target.stencil_load_op == RafxLoadOp::Clear {
                    gl_context.gl_clear_stencil(depth_target.clear_value.stencil as _)?;
                    clear_mask |= gles3_bindings::STENCIL_BUFFER_BIT;
                }

                has_stencil = true;
            }
        }

        if state.framebuffer_depth_bound && !has_depth {
            Self::unbind_framebuffer(gl_context, gles3_bindings::DEPTH_ATTACHMENT)?;
            state.framebuffer_depth_bound = false;
        }

        if state.framebuffer_stencil_bound && !has_stencil {
            Self::unbind_framebuffer(gl_context, gles3_bindings::STENCIL_ATTACHMENT)?;
            state.framebuffer_stencil_bound = false;
        }

        Self::do_set_viewport(
            gl_context,
            0,
            0,
            extents.width as _,
            extents.height as _,
            0.0,
            1.0,
        )?;

        Self::do_cmd_set_scissor(gl_context, 0, 0, extents.width, extents.height)?;

        let result = gl_context.gl_check_framebuffer_status(gles3_bindings::FRAMEBUFFER)?;
        if result != gles3_bindings::FRAMEBUFFER_COMPLETE {
            Err(format!(
                "Framebuffer Status is not FRAMEBUFFER_COMPLETE, result: {:#x}",
                result
            ))?;
        }

        if clear_mask != 0 {
            gl_context.gl_clear(clear_mask)?;
        }

        state.surface_size = Some(extents.to_2d());
        Ok(())
    }

    pub fn cmd_end_render_pass(&self) -> RafxResult<()> {
        let mut state = self.command_pool_state.borrow_mut();
        assert!(state.is_started);

        let gl_context = self.queue.device_context().gl_context();
        gl_context.gl_bind_framebuffer(gles3_bindings::FRAMEBUFFER, NONE_FRAMEBUFFER)?;

        state.surface_size = None;
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
        let state = self.command_pool_state.borrow();
        assert!(state.is_started);

        let gl_context = self.queue.device_context().gl_context();
        Self::do_set_viewport(
            gl_context,
            x as _,
            y as _,
            width as _,
            height as _,
            depth_min,
            depth_max,
        )
    }

    fn do_set_viewport(
        gl_context: &GlContext,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        depth_min: f32,
        depth_max: f32,
    ) -> RafxResult<()> {
        gl_context.gl_viewport(x, y, width, height)?;
        gl_context.gl_depth_rangef(depth_min, depth_max)
    }

    pub fn cmd_set_scissor(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> RafxResult<()> {
        let state = self.command_pool_state.borrow();
        assert!(state.is_started);

        let gl_context = self.queue.device_context().gl_context();
        Self::do_cmd_set_scissor(gl_context, x as _, y as _, width as _, height as _)
    }

    pub fn do_cmd_set_scissor(
        gl_context: &GlContext,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> RafxResult<()> {
        gl_context.gl_scissor(x as _, y as _, width as _, height as _)
    }

    pub fn cmd_set_stencil_reference_value(
        &self,
        value: u32,
    ) -> RafxResult<()> {
        let mut state = self.command_pool_state.borrow_mut();
        assert!(state.is_started);
        state.stencil_reference_value = value;

        let gl_context = self.queue.device_context().gl_context();
        if let Some(info) = &state.current_gl_pipeline_info {
            Self::do_set_stencil_compare_ref_mask(
                gl_context,
                &info.gl_depth_stencil_state,
                state.stencil_reference_value,
            )?;
        }

        Ok(())
    }

    // This logic is shared between cmd_set_stencil_reference_value and cmd_bind_pipeline
    fn do_set_stencil_compare_ref_mask(
        gl_context: &GlContext,
        state: &Gles3DepthStencilState,
        stencil_reference_value: u32,
    ) -> RafxResult<()> {
        if state.stencil_test_enable {
            gl_context.gl_stencil_func_separate(
                gles3_bindings::FRONT,
                state.front_stencil_compare_op,
                stencil_reference_value as _,
                !0,
            )?;
            gl_context.gl_stencil_func_separate(
                gles3_bindings::BACK,
                state.back_stencil_compare_op,
                stencil_reference_value as _,
                !0,
            )?;
        }

        Ok(())
    }

    pub fn cmd_bind_pipeline(
        &self,
        pipeline: &RafxPipelineGles3,
    ) -> RafxResult<()> {
        let mut state = self.command_pool_state.borrow_mut();
        assert!(state.is_started);

        // if let Some(previous_pipeline) = state.current_gl_pipeline_info {
        //     if pipeline_info.root_signature != previous_pipeline.root_signature {
        //         state.clear_bindings();
        //     }
        // }

        let pipeline_info = pipeline.gl_pipeline_info();
        state.current_gl_pipeline_info = Some(pipeline_info.clone());

        let gl_rasterizer_state = &pipeline_info.gl_rasterizer_state;
        let gl_depth_stencil_state = &pipeline_info.gl_depth_stencil_state;
        let gl_blend_state = &pipeline_info.gl_blend_state;

        let gl_context = self.queue.device_context().gl_context();
        gl_context.gl_use_program(pipeline.gl_program_id())?;

        if gl_rasterizer_state.cull_mode != gles3_bindings::NONE {
            gl_context.gl_enable(gles3_bindings::CULL_FACE)?;
            gl_context.gl_cull_face(gl_rasterizer_state.cull_mode)?;
            gl_context.gl_front_face(gl_rasterizer_state.front_face)?;
        } else {
            gl_context.gl_disable(gles3_bindings::CULL_FACE)?;
        }

        if gl_rasterizer_state.scissor_test {
            gl_context.gl_enable(gles3_bindings::SCISSOR_TEST)?;
        } else {
            gl_context.gl_disable(gles3_bindings::SCISSOR_TEST)?;
        }

        if gl_depth_stencil_state.depth_test_enable {
            gl_context.gl_enable(gles3_bindings::DEPTH_TEST)?;
            gl_context.gl_depth_mask(gl_depth_stencil_state.depth_write_enable)?;
            gl_context.gl_depth_func(gl_depth_stencil_state.depth_compare_op)?;
        } else {
            gl_context.gl_disable(gles3_bindings::DEPTH_TEST)?;
        }

        if gl_depth_stencil_state.stencil_test_enable {
            gl_context.gl_enable(gles3_bindings::STENCIL_TEST)?;
            gl_context.gl_stencil_mask(gl_depth_stencil_state.stencil_write_mask as _)?;

            Self::do_set_stencil_compare_ref_mask(
                gl_context,
                gl_depth_stencil_state,
                state.stencil_reference_value,
            )?;

            gl_context.gl_stencil_op_separate(
                gles3_bindings::FRONT,
                gl_depth_stencil_state.front_stencil_fail_op,
                gl_depth_stencil_state.front_depth_fail_op,
                gl_depth_stencil_state.front_stencil_pass_op,
            )?;

            gl_context.gl_stencil_op_separate(
                gles3_bindings::BACK,
                gl_depth_stencil_state.back_stencil_fail_op,
                gl_depth_stencil_state.back_depth_fail_op,
                gl_depth_stencil_state.back_stencil_pass_op,
            )?;
        } else {
            gl_context.gl_disable(gles3_bindings::STENCIL_TEST)?;
        }

        if gl_blend_state.enabled {
            gl_context.gl_enable(gles3_bindings::BLEND)?;
            gl_context.gl_blend_func_separate(
                gl_blend_state.src_factor,
                gl_blend_state.dst_factor,
                gl_blend_state.src_factor_alpha,
                gl_blend_state.dst_factor_alpha,
            )?;

            gl_context.gl_blend_equation_separate(
                gl_blend_state.blend_op,
                gl_blend_state.blend_op_alpha,
            )?;
        } else {
            gl_context.gl_disable(gles3_bindings::BLEND)?;
        }

        gl_context.gl_color_mask(
            gl_blend_state.color_flags.intersects(RafxColorFlags::RED),
            gl_blend_state.color_flags.intersects(RafxColorFlags::GREEN),
            gl_blend_state.color_flags.intersects(RafxColorFlags::BLUE),
            gl_blend_state.color_flags.intersects(RafxColorFlags::ALPHA),
        )?;

        Ok(())
    }

    pub fn cmd_bind_vertex_buffers(
        &self,
        first_binding: u32,
        bindings: &[RafxVertexBufferBinding],
    ) -> RafxResult<()> {
        let mut state = self.command_pool_state.borrow_mut();
        assert!(state.is_started);
        assert!(
            first_binding + bindings.len() as u32
                <= self
                    .queue
                    .device_context()
                    .device_info()
                    .max_vertex_attribute_count
        );

        let gl_pipeline_info = state.current_gl_pipeline_info.as_ref().unwrap().clone();
        let gl_context = self.queue.device_context().gl_context();

        // All currently enabled attributes, we may need to clear previously bound attributes and enable
        // newly bound attributes
        let mut attributes_in_use = state.vertex_attribute_enabled_bits;

        let mut binding_index = first_binding;
        for binding in bindings {
            // First, clear the attributes_in_use flags that were previously used with this binding
            // and the attribute metadata
            if let Some(bound_vertex_buffer) = &state.bound_vertex_buffers[binding_index as usize] {
                let bound_attribute_bits = bound_vertex_buffer.attribute_enabled_bits;
                for (i, attribute) in state.vertex_attributes.iter_mut().enumerate() {
                    if (bound_attribute_bits & (1 << i)) != 0 {
                        *attribute = None;
                    }
                }

                attributes_in_use = attributes_in_use & !bound_attribute_bits;
            }

            // Check that the buffer is declared as a vertex buffer
            let gl_buffer = binding.buffer.gles3_buffer().unwrap();
            if !gl_buffer
                .buffer_def()
                .resource_type
                .intersects(RafxResourceType::VERTEX_BUFFER)
            {
                Err("Buffers provided to cmd_bind_vertex_buffer must be vertex buffers")?;
            }

            // Store all the attributes associated with this vertex buffer, they will be set up
            // when we try to draw by calling ensure_vertex_bindings_up_to_date(). This is deferred
            // to support vertex offsets in cmd_draw_index()
            let mut attributes_in_use_per_binding = 0;
            for attribute in &gl_pipeline_info.gl_attributes {
                if attribute.buffer_index != binding_index {
                    // Skip attributes that don't belong to this buffer
                    continue;
                }

                // Cache the attribute metadata
                state.vertex_attributes[attribute.location as usize] = Some(attribute.clone());

                // Enable the flag for this attribute
                attributes_in_use |= 1 << attribute.location;
                attributes_in_use_per_binding |= 1 << attribute.location;
            }

            // Cache the flags of all enabled attributes
            state.bound_vertex_buffers[binding_index as usize] = Some(BoundVertexBuffer {
                buffer_id: gl_buffer.gl_buffer_id().unwrap(),
                byte_offset: binding.byte_offset as u32,
                attribute_enabled_bits: attributes_in_use_per_binding,
            });

            // Since we've changed the vertex buffer and not bound it, clear the currently bound
            // vertex offset. (update_vertex_attributes_in_use() below will deactivate unused attributes)
            state.currently_bound_vertex_offset[binding_index as usize] = None;

            binding_index += 1;
        }

        Self::update_vertex_attributes_in_use(gl_context, &mut *state, attributes_in_use)
    }

    fn ensure_vertex_bindings_up_to_date(
        gl_context: &GlContext,
        state: &mut CommandPoolGles3StateInner,
        vertex_offset: i32,
    ) -> RafxResult<()> {
        let mut unbind_buffer = false;

        // Check all vertex buffers have been bound with the given offset
        for (vertex_buffer_index, bound_vertex_buffer) in
            state.bound_vertex_buffers.iter_mut().enumerate()
        {
            if let Some(bound_vertex_buffer) = bound_vertex_buffer {
                // The buffer is bound correctly, skip it
                if state.currently_bound_vertex_offset[vertex_buffer_index] == Some(vertex_offset) {
                    continue;
                }

                // Bind the buffer and set up any attributes that should be pulled from it
                gl_context
                    .gl_bind_buffer(gles3_bindings::ARRAY_BUFFER, bound_vertex_buffer.buffer_id)?;
                for i in 0..state.vertex_attributes.len() {
                    if bound_vertex_buffer.attribute_enabled_bits & (1 << i) != 0 {
                        let attribute = state.vertex_attributes[i].as_ref().unwrap();
                        debug_assert!(attribute.buffer_index == vertex_buffer_index as u32);
                        debug_assert!((1 << i) & state.vertex_attribute_enabled_bits != 0);
                        let byte_offset = bound_vertex_buffer.byte_offset as i32
                            + attribute.byte_offset as i32
                            + (attribute.stride as i32 * vertex_offset);
                        gl_context.gl_vertex_attrib_pointer(
                            attribute.location,
                            attribute.channel_count as _,
                            attribute.gl_type,
                            attribute.is_normalized,
                            attribute.stride,
                            byte_offset,
                        )?;
                    }
                }
                unbind_buffer = true;

                // Either the attributes are unbound or we need to rebind them with a different offset
                // Store the offset this buffer is configured with
                state.currently_bound_vertex_offset[vertex_buffer_index] = Some(vertex_offset);
            }
        }

        if unbind_buffer {
            gl_context.gl_bind_buffer(gles3_bindings::ARRAY_BUFFER, NONE_BUFFER)?;
        }

        Ok(())
    }

    fn update_vertex_attributes_in_use(
        gl_context: &GlContext,
        state: &mut CommandPoolGles3StateInner,
        desired: AttributeEnabledBits,
    ) -> RafxResult<()> {
        for i in 0..state.vertex_attributes.len() as u32 {
            let is_enabled = (1 << i) & state.vertex_attribute_enabled_bits;
            let should_be_enabled = (1 << i) & desired;
            if is_enabled != should_be_enabled {
                if should_be_enabled != 0 {
                    gl_context.gl_enable_vertex_attrib_array(i)?;
                } else {
                    gl_context.gl_disable_vertex_attrib_array(i)?;
                }
            }
        }

        state.vertex_attribute_enabled_bits = desired;
        Ok(())
    }

    pub fn cmd_bind_index_buffer(
        &self,
        binding: &RafxIndexBufferBinding,
    ) -> RafxResult<()> {
        let mut state = self.command_pool_state.borrow_mut();
        assert!(state.is_started);

        let gl_context = self.queue.device_context().gl_context();

        if binding.index_type != RafxIndexType::Uint16 {
            unimplemented!("GL ES 2.0 only supports Uint16 index buffers");
        }

        let buffer = binding.buffer.gles3_buffer().unwrap();
        if !buffer
            .buffer_def()
            .resource_type
            .intersects(RafxResourceType::INDEX_BUFFER)
        {
            Err("Buffers provided to cmd_bind_index_buffer must be index buffers")?;
        }

        state.index_buffer_byte_offset = binding.byte_offset as u32;
        gl_context.gl_bind_buffer(
            gles3_bindings::ELEMENT_ARRAY_BUFFER,
            buffer.gl_buffer_id().unwrap(),
        )
    }

    pub fn cmd_bind_descriptor_set(
        &self,
        descriptor_set_array: &RafxDescriptorSetArrayGles3,
        index: u32,
    ) -> RafxResult<()> {
        let mut state = self.command_pool_state.borrow_mut();
        assert!(state.is_started);

        self.set_current_descriptor_set(
            &mut *state,
            descriptor_set_array.descriptor_set_array_data(),
            descriptor_set_array
                .root_signature()
                .gles3_root_signature()
                .unwrap(),
            descriptor_set_array.set_index(),
            index,
        );

        Ok(())
    }

    pub fn cmd_bind_descriptor_set_handle(
        &self,
        root_signature: &RafxRootSignatureGles3,
        set_index: u32,
        descriptor_set_handle: &RafxDescriptorSetHandleGles3,
    ) -> RafxResult<()> {
        let mut state = self.command_pool_state.borrow_mut();
        assert!(state.is_started);

        self.set_current_descriptor_set(
            &mut *state,
            descriptor_set_handle.descriptor_set_array_data(),
            root_signature,
            set_index,
            descriptor_set_handle.array_index(),
        );
        Ok(())
    }

    // This does not affect the program right away, we wait until we try to draw, then update the
    // program as necessary
    fn set_current_descriptor_set(
        &self,
        state: &mut CommandPoolGles3StateInner,
        data: &Arc<TrustCell<DescriptorSetArrayData>>,
        root_signature: &RafxRootSignatureGles3,
        set_index: u32,
        array_index: u32,
    ) {
        // If we bind a descriptor set with a different root signature, clear the other bindings
        if let Some(current_root_signature) = &state.bound_descriptor_sets_root_signature {
            if current_root_signature != root_signature {
                state.clear_bindings();
                state.bound_descriptor_sets_root_signature = Some(root_signature.clone());
            }
        } else {
            state.bound_descriptor_sets_root_signature = Some(root_signature.clone());
        }

        // Cache the info necessary to update bound programs later
        state.bound_descriptor_sets[set_index as usize] = Some(BoundDescriptorSet {
            data: data.clone(),
            array_index,
        });
        state.descriptor_sets_update_index[set_index as usize] += 1;
    }

    // Call right before drawing, this just checks that the program is up-to-date with the latest
    // bound descriptor sets
    fn ensure_pipeline_bindings_up_to_date(
        gl_context: &GlContext,
        state: &CommandPoolGles3StateInner,
    ) -> RafxResult<()> {
        let pipeline = state.current_gl_pipeline_info.as_ref().unwrap();
        let mut last_descriptor_updates = pipeline.last_descriptor_updates.borrow_mut();

        // If the program was previously bound by some other command pool, we can't assume it's in
        // the same state as before. Clear the last_descriptor_updates values to ensure that we push
        // all sets into the program state
        let mut last_bound_by_command_pool = pipeline.last_bound_by_command_pool.borrow_mut();
        if *last_bound_by_command_pool != state.id {
            *last_bound_by_command_pool = state.id;

            for set_index in 0..MAX_DESCRIPTOR_SET_LAYOUTS {
                last_descriptor_updates[set_index] = 0;
            }
        }

        if let Some(bound_descriptor_sets_root_signature) =
            &state.bound_descriptor_sets_root_signature
        {
            // Only update the program if the bound descriptor sets match the root signature
            if *bound_descriptor_sets_root_signature == pipeline.root_signature {
                for set_index in 0..MAX_DESCRIPTOR_SET_LAYOUTS {
                    if let Some(bound_descriptor_set) = &state.bound_descriptor_sets[set_index] {
                        if last_descriptor_updates[set_index]
                            < state.descriptor_sets_update_index[set_index]
                        {
                            Self::do_bind_descriptor_set(
                                gl_context,
                                pipeline,
                                &*bound_descriptor_set.data.borrow(),
                                set_index as u32,
                                bound_descriptor_set.array_index,
                            )?;

                            last_descriptor_updates[set_index] =
                                state.descriptor_sets_update_index[set_index];
                        }
                    }
                }
            }
        }

        Ok(())
    }

    // Does the actual descriptor set binding
    fn do_bind_descriptor_set(
        gl_context: &GlContext,
        pipeline_info: &Arc<Gles3PipelineInfo>,
        data: &DescriptorSetArrayData,
        set_index: u32,
        array_index: u32,
    ) -> RafxResult<()> {
        let root_signature = &pipeline_info.root_signature;
        let uniform_reflection_data = root_signature.uniform_reflection_data();
        for descriptor_index in &root_signature.inner.layouts[set_index as usize].descriptors {
            let descriptor = &root_signature.inner.descriptors[descriptor_index.0 as usize];

            match descriptor.resource_type {
                RafxResourceType::SAMPLER => {
                    // do nothing, we handle this when dealing with textures
                }
                RafxResourceType::TEXTURE | RafxResourceType::TEXTURE_READ_WRITE => {
                    // Find where the texture states begin for this resource in this descriptor set
                    let base_image_state_index = array_index * data.texture_states_per_set
                        + descriptor.descriptor_data_offset_in_set.unwrap();

                    // May occur if the texture is dead code and never sampled
                    if descriptor.sampler_descriptor_index.is_none() {
                        continue;
                    }

                    //
                    // The samplers are either within the RafxDescriptorSetArray's data or,
                    // if it's an immutable sampler, in the root signature itself
                    //
                    // We need to find a sampler here because GL ES 2.0 expects sampler state
                    // to be set per-texture
                    //
                    let sampler = match descriptor.sampler_descriptor_index.unwrap() {
                        RafxSamplerIndexGles3::Immutable(immutable_index) => {
                            &root_signature.inner.immutable_samplers[immutable_index as usize]
                                .sampler
                        }
                        RafxSamplerIndexGles3::Mutable(sampler_descriptor_index) => {
                            // Find the descriptor with the relevant sampler
                            let sampler_descriptor =
                                root_signature.descriptor(sampler_descriptor_index).unwrap();
                            // Find the samplers within the descriptor set's flattened array of
                            // all samplers
                            let sampler_index = (array_index * data.sampler_states_per_set
                                + sampler_descriptor.descriptor_data_offset_in_set.unwrap())
                                as usize;
                            &data.sampler_states[sampler_index].as_ref().unwrap().sampler
                        }
                    };

                    let layout = &root_signature.inner.layouts[set_index as usize];

                    for i in 0..descriptor.element_count {
                        if let Some(location) =
                            pipeline_info.resource_location(descriptor.descriptor_index, i)
                        {
                            let image_state_index = base_image_state_index + i;
                            let texture = &data.texture_states[image_state_index as usize]
                                .as_ref()
                                .expect("Tried to use unbound texture")
                                .texture;

                            let texture_unit_index: u32 = layout.texture_unit_offset
                                + descriptor.descriptor_data_offset_in_set.unwrap()
                                + i;

                            //println!("texture unit {} for location {:?}", texture_unit_index, location);
                            gl_context.gl_active_texture(texture_unit_index)?;

                            let target = texture.gl_target();

                            //TODO: Handle specific mip levels/array slices (GL_TEXTURE_BASE_LEVEL and GL_TEXTURE_MAX_LEVEL on sampler, ES3 only)

                            gl_context.gl_bind_texture(
                                target,
                                texture.gl_raw_image().gl_texture_id().unwrap(),
                            )?;

                            gl_type_util::set_uniform(
                                gl_context,
                                location,
                                &texture_unit_index,
                                gles3_bindings::INT,
                                1,
                            )?;

                            let min_filter = if texture.texture_def().mip_count > 1 {
                                sampler.inner.gl_mip_map_mode
                            } else {
                                sampler.inner.gl_min_filter
                            };

                            gl_context.gl_tex_parameteri(
                                target,
                                gles3_bindings::TEXTURE_MIN_FILTER,
                                min_filter as _,
                            )?;
                            gl_context.gl_tex_parameteri(
                                target,
                                gles3_bindings::TEXTURE_MAG_FILTER,
                                sampler.inner.gl_mag_filter as _,
                            )?;
                            gl_context.gl_tex_parameteri(
                                target,
                                gles3_bindings::TEXTURE_WRAP_S,
                                sampler.inner.gl_address_mode_s as _,
                            )?;
                            gl_context.gl_tex_parameteri(
                                target,
                                gles3_bindings::TEXTURE_WRAP_T,
                                sampler.inner.gl_address_mode_t as _,
                            )?;
                        }
                    }
                }
                RafxResourceType::UNIFORM_BUFFER => {
                    if let Some(uniform_index) = descriptor.uniform_index {
                        // Find where the buffers states begin for this resource in this descriptor set
                        let base_buffer_state_index = array_index * data.buffer_states_per_set
                            + descriptor.descriptor_data_offset_in_set.unwrap();
                        for i in 0..descriptor.element_count {
                            // Find the buffer state for this specific element of the resource
                            let buffer_state_index = base_buffer_state_index + i;
                            let buffer_state = data.buffer_states[buffer_state_index as usize]
                                .as_ref()
                                .unwrap();

                            // Get a ptr to the start of the uniform data we're binding
                            let uniform_data_ptr = unsafe {
                                buffer_state
                                    .buffer_contents
                                    .as_ref()
                                    .unwrap()
                                    .try_as_ptr()
                                    .expect("bound uniform buffer must be CPU-visible")
                                    .add(buffer_state.offset as usize)
                            };

                            let fields = uniform_reflection_data.uniform_fields(uniform_index);
                            for field in fields {
                                // Iterate through each member, updating the values
                                if let Some(location) =
                                    pipeline_info.uniform_member_location(field.field_index)
                                {
                                    let field_ref =
                                        unsafe { &*uniform_data_ptr.add(field.offset as usize) };

                                    gl_type_util::set_uniform(
                                        gl_context,
                                        location,
                                        field_ref,
                                        field.ty,
                                        field.element_count,
                                    )?;
                                }
                            }
                        }
                    }
                }
                RafxResourceType::BUFFER | RafxResourceType::BUFFER_READ_WRITE => {
                    unimplemented!("SSBOs are not supported in GL ES 2.0")
                }
                _ => unimplemented!("Unrecognized descriptor type in do_bind_descriptor_set"),
            }
        }

        Ok(())
    }

    pub fn cmd_draw(
        &self,
        vertex_count: u32,
        first_vertex: u32,
    ) -> RafxResult<()> {
        let mut state = self.command_pool_state.borrow_mut();
        assert!(state.is_started);

        let gl_context = self.queue.device_context().gl_context();
        Self::ensure_pipeline_bindings_up_to_date(gl_context, &*state)?;
        Self::ensure_vertex_bindings_up_to_date(gl_context, &mut *state, 0)?;
        let pipeline_info = state.current_gl_pipeline_info.as_ref().unwrap();

        gl_context.gl_draw_arrays(
            pipeline_info.gl_topology,
            first_vertex as _,
            vertex_count as _,
        )?;

        Ok(())
    }

    pub fn cmd_draw_instanced(
        &self,
        _vertex_count: u32,
        _first_vertex: u32,
        _instance_count: u32,
        _first_instance: u32,
    ) -> RafxResult<()> {
        unimplemented!("Instanced drawing not natively supported by GL ES 2.0");
    }

    pub fn cmd_draw_indexed(
        &self,
        index_count: u32,
        first_index: u32,
        vertex_offset: i32,
    ) -> RafxResult<()> {
        let mut state = self.command_pool_state.borrow_mut();
        assert!(state.is_started);

        let gl_context = self.queue.device_context().gl_context();
        Self::ensure_pipeline_bindings_up_to_date(gl_context, &*state)?;
        // glDrawElementsBaseVertex not supported in ES until 3.2
        Self::ensure_vertex_bindings_up_to_date(gl_context, &mut *state, vertex_offset)?;
        let pipeline_info = state.current_gl_pipeline_info.as_ref().unwrap();

        let index_byte_offset = first_index
            * (std::mem::size_of::<gles3_bindings::types::GLushort>() as u32)
            + state.index_buffer_byte_offset;

        gl_context.gl_draw_elements(
            pipeline_info.gl_topology,
            index_count as _,
            gles3_bindings::UNSIGNED_SHORT,
            index_byte_offset,
        )
    }

    pub fn cmd_draw_indexed_instanced(
        &self,
        _index_count: u32,
        _first_index: u32,
        _instance_count: u32,
        _first_instance: u32,
        _vertex_offset: i32,
    ) -> RafxResult<()> {
        unimplemented!("Instanced drawing not natively supported by GL ES 2.0");
    }

    pub fn cmd_dispatch(
        &self,
        _group_count_x: u32,
        _group_count_y: u32,
        _group_count_z: u32,
    ) -> RafxResult<()> {
        unimplemented!("Compute shaders not supported in GL ES 2.0");
    }

    pub fn cmd_resource_barrier(
        &self,
        _buffer_barriers: &[RafxBufferBarrier],
        _texture_barriers: &[RafxTextureBarrier],
    ) -> RafxResult<()> {
        // don't need to do anything
        Ok(())
    }

    pub fn cmd_copy_buffer_to_buffer(
        &self,
        src_buffer: &RafxBufferGles3,
        dst_buffer: &RafxBufferGles3,
        src_offset: u64,
        dst_offset: u64,
        size: u64,
    ) -> RafxResult<()> {
        let state = self.command_pool_state.borrow();
        assert!(state.is_started);

        let gl_context = self.queue.device_context().gl_context();

        let gl_target = dst_buffer.gl_target();
        gl_context.gl_bind_buffer(gl_target, dst_buffer.gl_buffer_id().unwrap())?;
        let src_data = unsafe {
            src_buffer
                .buffer_contents()
                .try_as_ptr()
                .expect("src buffer must be CPU-visible in cmd_copy_buffer_to_buffer")
                .add(src_offset as usize)
        };
        gl_context.gl_buffer_sub_data(gl_target, dst_offset as _, size, src_data)?;
        gl_context.gl_bind_buffer(gl_target, NONE_BUFFER)
    }

    pub fn cmd_copy_buffer_to_texture(
        &self,
        src_buffer: &RafxBufferGles3,
        dst_texture: &RafxTextureGles3,
        params: &RafxCmdCopyBufferToTextureParams,
    ) -> RafxResult<()> {
        let state = self.command_pool_state.borrow();
        assert!(state.is_started);

        let gl_context = self.queue.device_context().gl_context();

        let width = 1.max(dst_texture.texture_def().extents.width >> params.mip_level);
        let height = 1.max(dst_texture.texture_def().extents.height >> params.mip_level);

        let mut subtarget = dst_texture.gl_target();
        if subtarget == gles3_bindings::TEXTURE_CUBE_MAP {
            subtarget = array_layer_to_cube_map_target(params.array_layer);
        }

        let format_info = dst_texture.gl_format_info();

        let texture_id = dst_texture
            .gl_raw_image()
            .gl_texture_id()
            .ok_or("Cannot use cmd_copy_buffer_to_texture with swapchain image in GL ES 2.0")?;

        let buffer_ptr = unsafe {
            src_buffer
                .buffer_contents()
                .try_as_slice_with_offset(params.buffer_offset)
                .expect("src buffer must be CPU-visible in cmd_copy_buffer_to_texture")
        };

        gl_context.gl_bind_texture(dst_texture.gl_target(), texture_id)?;
        //TODO: Compressed texture support?
        gl_context.gl_tex_image_2d(
            subtarget,
            params.mip_level as _,
            format_info.gl_internal_format,
            width,
            height,
            0,
            format_info.gl_format,
            format_info.gl_type,
            Some(&buffer_ptr),
        )?;
        gl_context.gl_bind_texture(dst_texture.gl_target(), NONE_TEXTURE)
    }
}
