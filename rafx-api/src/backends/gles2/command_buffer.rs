use crate::gles2::{
    AttributeEnabledBits, BoundDescriptorSet, CommandPoolGles2State, CommandPoolGles2StateInner,
    DescriptorSetArrayData, GlContext, Gles2PipelineInfo, RafxBufferGles2, RafxCommandPoolGles2,
    RafxDescriptorSetArrayGles2, RafxDescriptorSetHandleGles2, RafxPipelineGles2, RafxQueueGles2,
    RafxRootSignatureGles2, RafxTextureGles2, NONE_BUFFER, NONE_TEXTURE,
};
use crate::{
    RafxBufferBarrier, RafxCmdCopyBufferToTextureParams, RafxColorRenderTargetBinding,
    RafxCommandBufferDef, RafxDepthStencilRenderTargetBinding, RafxExtents3D,
    RafxIndexBufferBinding, RafxIndexType, RafxLoadOp, RafxResourceType, RafxResult,
    RafxTextureBarrier, RafxVertexBufferBinding, MAX_DESCRIPTOR_SET_LAYOUTS,
};

use rafx_base::trust_cell::TrustCell;

use crate::gles2::conversions::Gles2DepthStencilState;
use crate::gles2::gles2_bindings;

use crate::backends::gles2::{RafxRawImageGles2, RafxSamplerIndexGles2};
use crate::gles2::gl_type_util;
use crate::gles2::gles2_bindings::types::GLenum;
use std::sync::Arc;

#[derive(Debug)]
pub struct RafxCommandBufferGles2 {
    queue: RafxQueueGles2,
    command_pool_state: CommandPoolGles2State,
}

impl RafxCommandBufferGles2 {
    pub fn new(
        command_pool: &RafxCommandPoolGles2,
        _command_buffer_def: &RafxCommandBufferDef,
    ) -> RafxResult<RafxCommandBufferGles2> {
        Ok(RafxCommandBufferGles2 {
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
        assert!(state.surface_size.is_none());

        let gl_context = self.queue.device_context().gl_context();
        Self::update_vertex_attributes_in_use(gl_context, &mut *state, 0)?;

        assert_eq!(state.vertex_attribute_enabled_bits, 0);

        state.is_started = false;
        state.current_gl_pipeline_info = None;
        state.stencil_reference_value = 0;
        state.index_buffer_byte_offset = 0;
        for offset in &mut state.vertex_buffer_byte_offsets {
            *offset = 0;
        }
        state.clear_bindings();

        Ok(())
    }

    pub fn return_to_pool(&self) -> RafxResult<()> {
        // don't need to do anything
        Ok(())
    }

    fn bind_framebuffer(
        gl_context: &GlContext,
        texture: &RafxTextureGles2,
        attachment: GLenum,
        mip_slice: Option<u8>,
    ) -> RafxResult<()> {
        match texture.gl_raw_image() {
            // RafxRawImageGl::Renderbuffer(id) => {
            //     assert!(mip_slice.is_none());
            //     gl_context.gl_bind_renderbuffer(gles20::RENDERBUFFER, *id)?;
            //     if *id != NONE_RENDERBUFFER {
            //         gl_context.gl_framebuffer_renderbuffer(
            //             gles20::FRAMEBUFFER,
            //             attachment,
            //             gles20::RENDERBUFFER,
            //             *id,
            //         )?;
            //     }
            //
            //     gl_context.gl_bind_renderbuffer(gles20::RENDERBUFFER, NONE_RENDERBUFFER)?;
            // }
            RafxRawImageGles2::Texture(id) => {
                //TODO: Handle cubemap
                let texture_target = gles2_bindings::TEXTURE_2D;
                gl_context.gl_bind_texture(texture_target, *id)?;
                gl_context.gl_framebuffer_texture(
                    gles2_bindings::FRAMEBUFFER,
                    attachment,
                    texture_target,
                    *id,
                    mip_slice.unwrap_or(0),
                )?;
                gl_context.gl_bind_texture(texture_target, NONE_TEXTURE)?;
            }
        }

        Ok(())
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

        gl_context.gl_bind_framebuffer(gles2_bindings::FRAMEBUFFER, state.framebuffer_id)?;

        for (index, render_target) in color_targets.iter().enumerate() {
            extents = render_target.texture.texture_def().extents;

            let gl_texture = render_target.texture.gles2_texture().unwrap();
            let attachment = gles2_bindings::COLOR_ATTACHMENT0 + index as u32;
            Self::bind_framebuffer(gl_context, gl_texture, attachment, render_target.mip_slice)?;

            // match gl_texture.gl_raw_image() {
            //     RafxRawImageGl::Renderbuffer(id) => {
            //         gl_context.gl_bind_renderbuffer(gles20::RENDERBUFFER, *id)?;
            //         gl_context.gl_framebuffer_renderbuffer(gles20::FRAMEBUFFER, attachment, gles20::RENDERBUFFER, *id)?;
            //         gl_context.gl_bind_renderbuffer(gles20::RENDERBUFFER, NONE_RENDERBUFFER)?;
            //     }
            //     RafxRawImageGl::Texture(id) => {
            //         //TODO: Handle cubemap
            //         let texture_target = gles20::TEXTURE_2D;
            //         gl_context.gl_bind_texture(texture_target, *id)?;
            //         gl_context.gl_framebuffer_texture(gles20::FRAMEBUFFER, attachment, gles20::TEXTURE_2D, *id, render_target.mip_slice.unwrap_or(0))?;
            //         gl_context.gl_bind_texture(texture_target, NONE_TEXTURE)?;
            //     }
            // }

            // let renderbuffer = gl_texture.gl_raw_image().gl_renderbuffer_id().unwrap();
            // gl_context.gl_bind_renderbuffer(gles20::RENDERBUFFER, renderbuffer)?;
            // if renderbuffer != NONE_RENDERBUFFER {
            //     gl_context.gl_framebuffer_renderbuffer(
            //         gles20::FRAMEBUFFER,
            //         attachment,
            //         gles20::RENDERBUFFER,
            //         renderbuffer,
            //     )?;
            // }

            if render_target.load_op == RafxLoadOp::Clear {
                let c = &render_target.clear_value.0;
                gl_context.gl_clear_color(c[0], c[1], c[2], c[3])?;
                clear_mask |= gles2_bindings::COLOR_BUFFER_BIT;
            }

            //gl_context.gl_bind_renderbuffer(gles20::RENDERBUFFER, NONE_RENDERBUFFER)?;
        }

        if let Some(depth_target) = depth_target {
            let format = depth_target.texture.texture_def().format;
            if format.has_depth() {
                extents = depth_target.texture.texture_def().extents;

                // let renderbuffer = depth_target
                //     .texture
                //     .gl_texture()
                //     .unwrap()
                //     .gl_raw_image()
                //     .gl_renderbuffer_id()
                //     .unwrap();
                // gl_context.gl_bind_renderbuffer(gles20::RENDERBUFFER, renderbuffer)?;
                // if renderbuffer != NONE_RENDERBUFFER {
                //     gl_context.gl_framebuffer_renderbuffer(
                //         gles20::FRAMEBUFFER,
                //         gles20::DEPTH_ATTACHMENT,
                //         gles20::RENDERBUFFER,
                //         renderbuffer,
                //     )?;
                // }

                let gl_texture = depth_target.texture.gles2_texture().unwrap();
                let attachment = gles2_bindings::DEPTH_ATTACHMENT;
                Self::bind_framebuffer(gl_context, gl_texture, attachment, depth_target.mip_slice)?;

                if depth_target.depth_load_op == RafxLoadOp::Clear {
                    gl_context.gl_clear_depthf(depth_target.clear_value.depth)?;
                    clear_mask |= gles2_bindings::DEPTH_BUFFER_BIT;
                }

                //gl_context.gl_bind_renderbuffer(gles20::RENDERBUFFER, NONE_RENDERBUFFER)?;
            }

            if format.has_stencil() {
                extents = depth_target.texture.texture_def().extents;

                // let renderbuffer = depth_target
                //     .texture
                //     .gl_texture()
                //     .unwrap()
                //     .gl_raw_image()
                //     .gl_renderbuffer_id()
                //     .unwrap();
                // gl_context.gl_bind_renderbuffer(gles20::RENDERBUFFER, renderbuffer)?;
                // if renderbuffer != NONE_RENDERBUFFER {
                //     gl_context.gl_framebuffer_renderbuffer(
                //         gles20::FRAMEBUFFER,
                //         gles20::STENCIL_ATTACHMENT,
                //         gles20::RENDERBUFFER,
                //         renderbuffer,
                //     )?;
                // }

                let gl_texture = depth_target.texture.gles2_texture().unwrap();
                let attachment = gles2_bindings::STENCIL_ATTACHMENT;
                Self::bind_framebuffer(gl_context, gl_texture, attachment, depth_target.mip_slice)?;

                if depth_target.stencil_load_op == RafxLoadOp::Clear {
                    gl_context.gl_clear_stencil(depth_target.clear_value.stencil as _)?;
                    clear_mask |= gles2_bindings::STENCIL_BUFFER_BIT;
                }

                //gl_context.gl_bind_renderbuffer(gles20::RENDERBUFFER, NONE_RENDERBUFFER)?;
            }
        }

        Self::do_set_viewport(
            gl_context,
            extents.height as _,
            0,
            0,
            extents.width as _,
            extents.height as _,
            0.0,
            1.0,
        )?;

        Self::do_cmd_set_scissor(
            gl_context,
            extents.height,
            0,
            0,
            extents.width,
            extents.height,
        )?;

        let result = gl_context.gl_check_framebuffer_status(gles2_bindings::FRAMEBUFFER)?;
        if result != gles2_bindings::FRAMEBUFFER_COMPLETE {
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
            state.surface_size.unwrap().height as _,
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
        surface_height: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        depth_min: f32,
        depth_max: f32,
    ) -> RafxResult<()> {
        let y_offset = surface_height - y - height;
        gl_context.gl_viewport(x, y_offset, width, height)?;
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
        Self::do_cmd_set_scissor(
            gl_context,
            state.surface_size.unwrap().height as _,
            x as _,
            y as _,
            width as _,
            height as _,
        )
    }

    pub fn do_cmd_set_scissor(
        gl_context: &GlContext,
        surface_height: u32,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> RafxResult<()> {
        let y_offset = surface_height - y - height;

        gl_context.gl_scissor(x as _, y_offset as _, width as _, height as _)
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
        state: &Gles2DepthStencilState,
        stencil_reference_value: u32,
    ) -> RafxResult<()> {
        if state.stencil_test_enable {
            gl_context.gl_stencil_func_separate(
                gles2_bindings::FRONT,
                state.front_stencil_compare_op,
                stencil_reference_value as _,
                !0,
            )?;
            gl_context.gl_stencil_func_separate(
                gles2_bindings::BACK,
                state.back_stencil_compare_op,
                stencil_reference_value as _,
                !0,
            )?;
        }

        Ok(())
    }

    pub fn cmd_bind_pipeline(
        &self,
        pipeline: &RafxPipelineGles2,
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

        if gl_rasterizer_state.cull_mode != gles2_bindings::NONE {
            gl_context.gl_enable(gles2_bindings::CULL_FACE)?;
            gl_context.gl_cull_face(gl_rasterizer_state.cull_mode)?;
            gl_context.gl_front_face(gl_rasterizer_state.front_face)?;
        } else {
            gl_context.gl_disable(gles2_bindings::CULL_FACE)?;
        }

        if gl_rasterizer_state.scissor_test {
            gl_context.gl_enable(gles2_bindings::SCISSOR_TEST)?;
        } else {
            gl_context.gl_disable(gles2_bindings::SCISSOR_TEST)?;
        }

        if gl_depth_stencil_state.depth_test_enable {
            gl_context.gl_enable(gles2_bindings::DEPTH_TEST)?;
            gl_context.gl_depth_mask(gl_depth_stencil_state.depth_write_enable)?;
            gl_context.gl_depth_func(gl_depth_stencil_state.depth_compare_op)?;
        } else {
            gl_context.gl_disable(gles2_bindings::DEPTH_TEST)?;
        }

        if gl_depth_stencil_state.stencil_test_enable {
            gl_context.gl_enable(gles2_bindings::STENCIL_TEST)?;
            gl_context.gl_stencil_mask(gl_depth_stencil_state.stencil_write_mask as _)?;

            Self::do_set_stencil_compare_ref_mask(
                gl_context,
                gl_depth_stencil_state,
                state.stencil_reference_value,
            )?;

            gl_context.gl_stencil_op_separate(
                gles2_bindings::FRONT,
                gl_depth_stencil_state.front_stencil_fail_op,
                gl_depth_stencil_state.front_depth_fail_op,
                gl_depth_stencil_state.front_stencil_pass_op,
            )?;

            gl_context.gl_stencil_op_separate(
                gles2_bindings::BACK,
                gl_depth_stencil_state.back_stencil_fail_op,
                gl_depth_stencil_state.back_depth_fail_op,
                gl_depth_stencil_state.back_stencil_pass_op,
            )?;
        } else {
            gl_context.gl_disable(gles2_bindings::STENCIL_TEST)?;
        }

        if gl_blend_state.enabled {
            gl_context.gl_enable(gles2_bindings::BLEND)?;
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
            gl_context.gl_disable(gles2_bindings::BLEND)?;
        }

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

        let mut attributes_in_use = 0;

        let mut binding_index = first_binding;
        for binding in bindings {
            let gl_buffer = binding.buffer.gles2_buffer().unwrap();
            assert_eq!(gl_buffer.gl_target(), gles2_bindings::ARRAY_BUFFER);

            // Bind the vertex buffer
            gl_context.gl_bind_buffer(gl_buffer.gl_target(), gl_buffer.gl_buffer_id().unwrap())?;
            state.vertex_buffer_byte_offsets[binding_index as usize] = binding.byte_offset as u32;

            // Setup all the attributes associated with this vertex buffer
            for attribute in &gl_pipeline_info.gl_attributes {
                if attribute.buffer_index != binding_index {
                    continue;
                }

                let byte_offset = binding.byte_offset as u32 + attribute.byte_offset;
                gl_context.gl_vertex_attrib_pointer(
                    attribute.location,
                    attribute.channel_count as _,
                    attribute.gl_type,
                    attribute.is_normalized,
                    attribute.stride,
                    byte_offset,
                )?;

                attributes_in_use |= 1 << attribute.location;
            }

            binding_index += 1;
        }

        Self::update_vertex_attributes_in_use(gl_context, &mut *state, attributes_in_use)
    }

    fn update_vertex_attributes_in_use(
        gl_context: &GlContext,
        state: &mut CommandPoolGles2StateInner,
        desired: AttributeEnabledBits,
    ) -> RafxResult<()> {
        for i in 0..state.vertex_buffer_byte_offsets.len() as u32 {
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

        let buffer = binding.buffer.gles2_buffer().unwrap();
        if buffer.gl_target() != gles2_bindings::ELEMENT_ARRAY_BUFFER {
            Err("Buffers provided to cmd_bind_index_buffer must be index buffers")?;
        }

        state.index_buffer_byte_offset = binding.byte_offset as u32;
        gl_context.gl_bind_buffer(
            gles2_bindings::ELEMENT_ARRAY_BUFFER,
            buffer.gl_buffer_id().unwrap(),
        )
    }

    pub fn cmd_bind_descriptor_set(
        &self,
        descriptor_set_array: &RafxDescriptorSetArrayGles2,
        index: u32,
    ) -> RafxResult<()> {
        let mut state = self.command_pool_state.borrow_mut();
        assert!(state.is_started);

        self.set_current_descriptor_set(
            &mut *state,
            descriptor_set_array.descriptor_set_array_data(),
            descriptor_set_array
                .root_signature()
                .gles2_root_signature()
                .unwrap(),
            descriptor_set_array.set_index(),
            index,
        );

        Ok(())
    }

    pub fn cmd_bind_descriptor_set_handle(
        &self,
        root_signature: &RafxRootSignatureGles2,
        set_index: u32,
        descriptor_set_handle: &RafxDescriptorSetHandleGles2,
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
        state: &mut CommandPoolGles2StateInner,
        data: &Arc<TrustCell<DescriptorSetArrayData>>,
        root_signature: &RafxRootSignatureGles2,
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
        state: &CommandPoolGles2StateInner,
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
        pipeline_info: &Arc<Gles2PipelineInfo>,
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
                    if let Some(location) =
                        pipeline_info.resource_location(descriptor.descriptor_index)
                    {
                        // Find where the texture states begin for this resource in this descriptor set
                        let base_image_state_index = array_index * data.texture_states_per_set
                            + descriptor.descriptor_data_offset_in_set.unwrap();

                        //
                        // The samplers are either within the RafxDescriptorSetArray's data or,
                        // if it's an immutable sampler, in the root signature itself
                        //
                        // We need to find a sampler here because GL ES 2.0 expects sampler state
                        // to be set per-texture
                        //
                        let mut immutable_samplers = None;
                        let mut mutable_samplers = None;
                        match descriptor.sampler_descriptor_index.unwrap() {
                            RafxSamplerIndexGles2::Immutable(immutable_index) => {
                                immutable_samplers = Some(
                                    &root_signature.inner.immutable_samplers
                                        [immutable_index as usize],
                                )
                            }
                            RafxSamplerIndexGles2::Mutable(sampler_descriptor_index) => {
                                // Find the descriptor with the relevant sampler
                                let sampler_descriptor =
                                    root_signature.descriptor(sampler_descriptor_index).unwrap();
                                // Find the samplers within the descriptor set's flattened array of
                                // all samplers
                                let first = (array_index * data.sampler_states_per_set
                                    + sampler_descriptor.descriptor_data_offset_in_set.unwrap())
                                    as usize;
                                let last = first + descriptor.element_count as usize;
                                mutable_samplers = Some(&data.sampler_states[first..last]);
                            }
                        }

                        for i in 0..descriptor.element_count {
                            let image_state_index = base_image_state_index + i;
                            let texture = &data.texture_states[image_state_index as usize]
                                .as_ref()
                                .expect("Tried to use unbound texture")
                                .texture;

                            let sampler = match descriptor.sampler_descriptor_index.unwrap() {
                                RafxSamplerIndexGles2::Immutable(_) => {
                                    &immutable_samplers.unwrap().samplers[i as usize]
                                }
                                RafxSamplerIndexGles2::Mutable(_) => {
                                    &mutable_samplers.unwrap()[i as usize]
                                        .as_ref()
                                        .expect("Tried to use unbound sampler")
                                        .sampler
                                }
                            };

                            gl_context.gl_active_texture(descriptor.texture_index.unwrap())?;
                            let target = texture.gl_target();
                            //TODO: handle cube map
                            //TODO: Handle specific mip levels/array slices (GL_TEXTURE_BASE_LEVEL and GL_TEXTURE_MAX_LEVEL on sampler, ES3 only)
                            gl_context.gl_bind_texture(
                                target,
                                texture.gl_raw_image().gl_texture_id().unwrap(),
                            )?;

                            gl_type_util::set_uniform(
                                gl_context,
                                location,
                                &descriptor.texture_index.unwrap(),
                                gles2_bindings::SAMPLER_2D,
                                1,
                            )?;

                            let min_filter = if texture.texture_def().mip_count > 1 {
                                sampler.inner.gl_mip_map_mode
                            } else {
                                sampler.inner.gl_min_filter
                            };

                            gl_context.gl_tex_parameteri(
                                target,
                                gles2_bindings::TEXTURE_MIN_FILTER,
                                min_filter as _,
                            )?;
                            gl_context.gl_tex_parameteri(
                                target,
                                gles2_bindings::TEXTURE_MAG_FILTER,
                                sampler.inner.gl_mag_filter as _,
                            )?;
                            gl_context.gl_tex_parameteri(
                                target,
                                gles2_bindings::TEXTURE_WRAP_S,
                                sampler.inner.gl_address_mode_s as _,
                            )?;
                            gl_context.gl_tex_parameteri(
                                target,
                                gles2_bindings::TEXTURE_WRAP_T,
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
                                    .as_ptr()
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
        let state = self.command_pool_state.borrow();
        assert!(state.is_started);

        let gl_context = self.queue.device_context().gl_context();
        let pipeline_info = state.current_gl_pipeline_info.as_ref().unwrap();
        Self::ensure_pipeline_bindings_up_to_date(gl_context, &*state)?;

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
        let state = self.command_pool_state.borrow();
        assert!(state.is_started);

        let gl_context = self.queue.device_context().gl_context();
        let pipeline_info = state.current_gl_pipeline_info.as_ref().unwrap();
        Self::ensure_pipeline_bindings_up_to_date(gl_context, &*state)?;

        if vertex_offset > 0 {
            unimplemented!("GL ES 2.0 does not support vertex offsets during glDrawElements");
        }

        let offset = first_index * (std::mem::size_of::<gles2_bindings::types::GLushort>() as u32)
            + state.index_buffer_byte_offset;
        gl_context.gl_draw_elements(
            pipeline_info.gl_topology,
            index_count as _,
            gles2_bindings::UNSIGNED_SHORT,
            offset,
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
        src_buffer: &RafxBufferGles2,
        dst_buffer: &RafxBufferGles2,
        src_offset: u64,
        dst_offset: u64,
        size: u64,
    ) -> RafxResult<()> {
        let state = self.command_pool_state.borrow();
        assert!(state.is_started);

        let gl_context = self.queue.device_context().gl_context();

        gl_context.gl_bind_buffer(dst_buffer.gl_target(), dst_buffer.gl_buffer_id().unwrap())?;
        let src_data = unsafe {
            src_buffer
                .buffer_contents()
                .as_ref()
                .unwrap()
                .as_ptr()
                .add(src_offset as usize)
        };
        gl_context.gl_buffer_sub_data(dst_buffer.gl_target(), dst_offset as _, size, src_data)?;
        gl_context.gl_bind_buffer(dst_buffer.gl_target(), NONE_BUFFER)
    }

    pub fn cmd_copy_buffer_to_texture(
        &self,
        src_buffer: &RafxBufferGles2,
        dst_texture: &RafxTextureGles2,
        params: &RafxCmdCopyBufferToTextureParams,
    ) -> RafxResult<()> {
        let state = self.command_pool_state.borrow();
        assert!(state.is_started);

        let gl_context = self.queue.device_context().gl_context();

        let width = 1.max(dst_texture.texture_def().extents.width >> params.mip_level);
        let height = 1.max(dst_texture.texture_def().extents.height >> params.mip_level);

        let mut target = dst_texture.gl_target();
        if target == gles2_bindings::TEXTURE_CUBE_MAP {
            match params.array_layer {
                0 => target = gles2_bindings::TEXTURE_CUBE_MAP_POSITIVE_X,
                1 => target = gles2_bindings::TEXTURE_CUBE_MAP_NEGATIVE_X,
                2 => target = gles2_bindings::TEXTURE_CUBE_MAP_POSITIVE_Y,
                3 => target = gles2_bindings::TEXTURE_CUBE_MAP_NEGATIVE_Y,
                4 => target = gles2_bindings::TEXTURE_CUBE_MAP_POSITIVE_Z,
                5 => target = gles2_bindings::TEXTURE_CUBE_MAP_NEGATIVE_Z,
                _ => unimplemented!("GL ES 2.0 does not support more than 6 images for a cubemap"),
            }
        }

        let format_info = dst_texture.gl_format_info();

        //TODO: Compressed texture support?
        let texture_id = dst_texture
            .gl_raw_image()
            .gl_texture_id()
            .ok_or("Cannot use cmd_copy_buffer_to_texture with swapchain image in GL ES 2.0")?;

        let buffer_contents = src_buffer
            .buffer_contents()
            .as_ref()
            .ok_or("Buffer used by cmd_copy_buffer_to_texture in GL ES 2.0 must be CPU-visible")?;
        let buffer_ptr = unsafe { buffer_contents.as_slice() };

        gl_context.gl_bind_texture(target, texture_id)?;
        gl_context.gl_tex_image_2d(
            target,
            params.mip_level as _,
            format_info.gl_internal_format,
            width,
            height,
            0,
            format_info.gl_format,
            format_info.gl_type,
            Some(buffer_ptr),
        )?;
        gl_context.gl_bind_texture(target, NONE_TEXTURE)
    }
}
