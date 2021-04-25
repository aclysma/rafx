use crate::gl::{
    BoundDescriptorSet, CommandPoolGlState, CommandPoolGlStateInner, DescriptorSetArrayData,
    GlContext, GlPipelineInfo, RafxBufferGl, RafxCommandPoolGl, RafxDescriptorSetArrayGl,
    RafxDescriptorSetHandleGl, RafxPipelineGl, RafxQueueGl, RafxRootSignatureGl, RafxTextureGl,
    NONE_BUFFER, NONE_RENDERBUFFER, NONE_TEXTURE,
};
use crate::{
    RafxBufferBarrier, RafxCmdCopyBufferToTextureParams, RafxColorRenderTargetBinding,
    RafxCommandBufferDef, RafxDepthStencilRenderTargetBinding, RafxExtents3D,
    RafxIndexBufferBinding, RafxIndexType, RafxLoadOp, RafxResourceType, RafxResult,
    RafxTextureBarrier, RafxVertexBufferBinding, MAX_DESCRIPTOR_SET_LAYOUTS,
};

use rafx_base::trust_cell::TrustCell;

use crate::gl::conversions::GlDepthStencilState;
use crate::gl::gles20;

use crate::backends::gl::RafxRawImageGl;
use crate::gl::gl_type_util;
use crate::gl::gles20::types::GLenum;
use std::sync::Arc;

#[derive(Debug)]
pub struct RafxCommandBufferGl {
    queue: RafxQueueGl,
    command_pool_state: CommandPoolGlState,
}

impl RafxCommandBufferGl {
    pub fn new(
        command_pool: &RafxCommandPoolGl,
        _command_buffer_def: &RafxCommandBufferDef,
    ) -> RafxResult<RafxCommandBufferGl> {
        Ok(RafxCommandBufferGl {
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
        texture: &RafxTextureGl,
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
            RafxRawImageGl::Texture(id) => {
                //TODO: Handle cubemap
                let texture_target = gles20::TEXTURE_2D;
                gl_context.gl_bind_texture(texture_target, *id)?;
                gl_context.gl_framebuffer_texture(
                    gles20::FRAMEBUFFER,
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

        if color_targets.is_empty() && depth_target.is_none() {
            Err("No color or depth target supplied to cmd_begin_render_pass")?;
        }

        let gl_context = self.queue.device_context().gl_context();
        let mut clear_mask = 0;
        let mut extents = RafxExtents3D::default();

        gl_context.gl_bind_framebuffer(gles20::FRAMEBUFFER, state.framebuffer_id)?;

        for (index, render_target) in color_targets.iter().enumerate() {
            extents = render_target.texture.texture_def().extents;

            let gl_texture = render_target.texture.gl_texture().unwrap();
            let attachment = gles20::COLOR_ATTACHMENT0 + index as u32;
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
                clear_mask |= gles20::COLOR_BUFFER_BIT;
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

                let gl_texture = depth_target.texture.gl_texture().unwrap();
                let attachment = gles20::DEPTH_ATTACHMENT;
                Self::bind_framebuffer(gl_context, gl_texture, attachment, depth_target.mip_slice)?;

                if depth_target.depth_load_op == RafxLoadOp::Clear {
                    gl_context.gl_clear_depthf(depth_target.clear_value.depth)?;
                    clear_mask |= gles20::DEPTH_BUFFER_BIT;
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

                let gl_texture = depth_target.texture.gl_texture().unwrap();
                let attachment = gles20::STENCIL_ATTACHMENT;
                Self::bind_framebuffer(gl_context, gl_texture, attachment, depth_target.mip_slice)?;

                if depth_target.stencil_load_op == RafxLoadOp::Clear {
                    gl_context.gl_clear_stencil(depth_target.clear_value.stencil as _)?;
                    clear_mask |= gles20::STENCIL_BUFFER_BIT;
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
            1.0
        )?;

        let result = gl_context.gl_check_framebuffer_status(gles20::FRAMEBUFFER)?;
        if result != gles20::FRAMEBUFFER_COMPLETE {
            Err(format!("Framebuffer Status is not FRAMEBUFFER_COMPLETE, result: {:#x}", result))?;
        }

        if clear_mask != 0 {
            gl_context.gl_clear(clear_mask)?;
        }

        state.surface_size = Some(extents.to_2d());

        std::mem::drop(state);
        // self.cmd_set_viewport(
        //     0.0,
        //     0.0,
        //     extents.width as f32,
        //     extents.height as f32,
        //     0.0,
        //     1.0,
        // )?;

        self.cmd_set_scissor(0, 0, extents.width, extents.height)
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
            depth_max
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
        let y_offset = state.surface_size.unwrap().height - y - height;

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
        state: &GlDepthStencilState,
        stencil_reference_value: u32,
    ) -> RafxResult<()> {
        if state.stencil_test_enable {
            gl_context.gl_stencil_func_separate(
                gles20::FRONT,
                state.front_stencil_compare_op,
                stencil_reference_value as _,
                !0,
            )?;
            gl_context.gl_stencil_func_separate(
                gles20::BACK,
                state.back_stencil_compare_op,
                stencil_reference_value as _,
                !0,
            )?;
        }

        Ok(())
    }

    pub fn cmd_bind_pipeline(
        &self,
        pipeline: &RafxPipelineGl,
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

        let max_attribs = self
            .queue
            .device_context()
            .device_info()
            .max_vertex_attribute_count;
        for i in 0..max_attribs {
            gl_context.gl_disable_vertex_attrib_array(i)?;
        }

        if gl_rasterizer_state.cull_mode != gles20::NONE {
            gl_context.gl_enable(gles20::CULL_FACE)?;
            gl_context.gl_cull_face(gl_rasterizer_state.cull_mode)?;
            gl_context.gl_front_face(gl_rasterizer_state.front_face)?;
        } else {
            gl_context.gl_disable(gles20::CULL_FACE)?;
        }

        if gl_rasterizer_state.scissor_test {
            gl_context.gl_enable(gles20::SCISSOR_TEST)?;
        } else {
            gl_context.gl_disable(gles20::SCISSOR_TEST)?;
        }

        if gl_depth_stencil_state.depth_test_enable {
            gl_context.gl_enable(gles20::DEPTH_TEST)?;
            gl_context.gl_depth_mask(gl_depth_stencil_state.depth_write_enable)?;
            gl_context.gl_depth_func(gl_depth_stencil_state.depth_compare_op)?;
        } else {
            gl_context.gl_disable(gles20::DEPTH_TEST)?;
        }

        if gl_depth_stencil_state.stencil_test_enable {
            gl_context.gl_enable(gles20::STENCIL_TEST)?;
            gl_context.gl_stencil_mask(gl_depth_stencil_state.stencil_write_mask as _)?;

            Self::do_set_stencil_compare_ref_mask(
                gl_context,
                gl_depth_stencil_state,
                state.stencil_reference_value,
            )?;

            gl_context.gl_stencil_op_separate(
                gles20::FRONT,
                gl_depth_stencil_state.front_stencil_fail_op,
                gl_depth_stencil_state.front_depth_fail_op,
                gl_depth_stencil_state.front_stencil_pass_op,
            )?;

            gl_context.gl_stencil_op_separate(
                gles20::BACK,
                gl_depth_stencil_state.back_stencil_fail_op,
                gl_depth_stencil_state.back_depth_fail_op,
                gl_depth_stencil_state.back_stencil_pass_op,
            )?;
        } else {
            gl_context.gl_disable(gles20::STENCIL_TEST)?;
        }

        if gl_blend_state.enabled {
            gl_context.gl_enable(gles20::BLEND)?;
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
            gl_context.gl_disable(gles20::BLEND)?;
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

        let mut binding_index = first_binding;
        for binding in bindings {
            let gl_buffer = binding.buffer.gl_buffer().unwrap();
            assert_eq!(gl_buffer.gl_target(), gles20::ARRAY_BUFFER);

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

                gl_context.gl_enable_vertex_attrib_array(attribute.location)?;
            }

            binding_index += 1;
        }

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
            Err("GL ES 2.0 only supports Uint16 index buffers")?;
        }

        let buffer = binding.buffer.gl_buffer().unwrap();
        if buffer.gl_target() != gles20::ELEMENT_ARRAY_BUFFER {
            Err("Buffers provided to cmd_bind_index_buffer must be index buffers")?;
        }

        state.index_buffer_byte_offset = binding.byte_offset as u32;
        gl_context.gl_bind_buffer(gles20::ELEMENT_ARRAY_BUFFER, buffer.gl_buffer_id().unwrap())
    }

    pub fn cmd_bind_descriptor_set(
        &self,
        descriptor_set_array: &RafxDescriptorSetArrayGl,
        index: u32,
    ) -> RafxResult<()> {
        let mut state = self.command_pool_state.borrow_mut();
        assert!(state.is_started);

        self.set_current_descriptor_set(
            &mut *state,
            descriptor_set_array.descriptor_set_array_data(),
            descriptor_set_array
                .root_signature()
                .gl_root_signature()
                .unwrap(),
            descriptor_set_array.set_index(),
            index,
        );

        Ok(())
    }

    pub fn cmd_bind_descriptor_set_handle(
        &self,
        root_signature: &RafxRootSignatureGl,
        set_index: u32,
        descriptor_set_handle: &RafxDescriptorSetHandleGl,
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
        state: &mut CommandPoolGlStateInner,
        data: &Arc<TrustCell<DescriptorSetArrayData>>,
        root_signature: &RafxRootSignatureGl,
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
        state: &CommandPoolGlStateInner,
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
        pipeline_info: &Arc<GlPipelineInfo>,
        data: &DescriptorSetArrayData,
        set_index: u32,
        array_index: u32,
    ) -> RafxResult<()> {
        let root_signature = &pipeline_info.root_signature;
        let uniform_reflection_data = root_signature.uniform_reflection_data();
        for descriptor_index in &root_signature.inner.layouts[set_index as usize].descriptors {
            let descriptor = &root_signature.inner.descriptors[descriptor_index.0 as usize];

            match descriptor.resource_type {
                RafxResourceType::BUFFER | RafxResourceType::BUFFER_READ_WRITE => {
                    let data_offset = descriptor.descriptor_data_offset_in_set.unwrap();
                    for i in 0..descriptor.element_count {
                        let buffer_state = data.buffer_states[(data_offset + i) as usize]
                            .as_ref()
                            .unwrap();

                        // let base_offset = buffer_state.offset;
                        // let data = unsafe {
                        //     buffer_state.buffer_contents.as_ref().unwrap().as_slice()
                        // };
                        //
                        // let location = root_signature.resource_location(program_index, descriptor.descriptor_index);
                        // if let Some(location) = location {
                        //     gl_type_util::set_uniform(gl_context, location, data, descriptor.gl_type, descriptor.element_count)?;
                        // }
                        unimplemented!()
                    }
                }
                RafxResourceType::TEXTURE | RafxResourceType::TEXTURE_READ_WRITE => {
                    unimplemented!()
                }
                RafxResourceType::UNIFORM_BUFFER => {
                    let uniform_index =
                        root_signature.uniform_index(descriptor.descriptor_index);

                    if let Some(uniform_index) = uniform_index {
                        // Find where the buffers states begin for this resource in this descriptor set
                        let base_buffer_state_index = array_index * data.buffer_states_per_set + descriptor.descriptor_data_offset_in_set.unwrap();
                        for i in 0..descriptor.element_count {
                            // Find the buffer state for this specific element of the resource
                            let buffer_state_index = base_buffer_state_index + i;
                            let buffer_state = data.buffer_states[buffer_state_index as usize]
                                .as_ref()
                                .unwrap();

                            // Get a ptr to the start of the uniform data we're binding
                            let uniform_data_ptr =
                                unsafe { buffer_state.buffer_contents.as_ref().unwrap().as_ptr().add(buffer_state.offset as usize) };

                            let fields = uniform_reflection_data.uniform_fields(uniform_index);
                            for field in fields {
                                // Iterate through each member, updating the values
                                if let Some(location) =
                                    pipeline_info.uniform_member_location(field.field_index)
                                {
                                    let field_ref = unsafe {
                                        &*uniform_data_ptr.add(field.offset as usize)
                                    };

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

        let offset = first_index * (std::mem::size_of::<gles20::types::GLushort>() as u32)
            + state.index_buffer_byte_offset;
        gl_context.gl_draw_elements(
            pipeline_info.gl_topology,
            index_count as _,
            gles20::UNSIGNED_SHORT,
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
        unimplemented!("Compute shaders not supported in GL ES");
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
        src_buffer: &RafxBufferGl,
        dst_buffer: &RafxBufferGl,
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
        src_buffer: &RafxBufferGl,
        dst_texture: &RafxTextureGl,
        params: &RafxCmdCopyBufferToTextureParams,
    ) -> RafxResult<()> {
        let state = self.command_pool_state.borrow();
        assert!(state.is_started);

        let gl_context = self.queue.device_context().gl_context();

        let width = 1.max(dst_texture.texture_def().extents.width >> params.mip_level);
        let height = 1.max(dst_texture.texture_def().extents.height >> params.mip_level);

        let mut target = dst_texture.gl_target();
        if target == gles20::TEXTURE_CUBE_MAP {
            match params.array_layer {
                0 => target = gles20::TEXTURE_CUBE_MAP_POSITIVE_X,
                1 => target = gles20::TEXTURE_CUBE_MAP_NEGATIVE_X,
                2 => target = gles20::TEXTURE_CUBE_MAP_POSITIVE_Y,
                3 => target = gles20::TEXTURE_CUBE_MAP_NEGATIVE_Y,
                4 => target = gles20::TEXTURE_CUBE_MAP_POSITIVE_Z,
                5 => target = gles20::TEXTURE_CUBE_MAP_NEGATIVE_Z,
                _ => return Err("GL ES 2.0 does not support more than 6 images for a cubemap")?,
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
