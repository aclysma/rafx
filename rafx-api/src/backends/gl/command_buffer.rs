use crate::gl::{DescriptorSetArrayData, RafxBufferGl, RafxCommandPoolGl, RafxDescriptorSetArrayGl, RafxDescriptorSetHandleGl, RafxPipelineGl, RafxQueueGl, RafxRootSignatureGl, RafxTextureGl, CommandPoolGlState, NONE_RENDERBUFFER, GlContext, CommandPoolGlStateInner, BoundDescriptorSet, GlPipelineInfo};
use crate::{RafxBufferBarrier, RafxCmdCopyBufferToTextureParams, RafxColorRenderTargetBinding, RafxCommandBufferDef, RafxDepthStencilRenderTargetBinding, RafxExtents3D, RafxIndexBufferBinding, RafxIndexType, RafxLoadOp, RafxPipelineType, RafxResourceState, RafxResult, RafxTextureBarrier, RafxVertexBufferBinding, RafxExtents2D, RafxResourceType, MAX_DESCRIPTOR_SET_LAYOUTS};
use fnv::FnvHashSet;

use rafx_base::trust_cell::TrustCell;

use crate::gl::gles20;
use crate::gl::conversions::GlDepthStencilState;

use crate::gl::gl_type_util;
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

        Ok(())
    }

    pub fn return_to_pool(&self) -> RafxResult<()> {
        // don't need to do anything
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

        for (index, render_target) in color_targets.iter().enumerate() {
            extents = render_target.texture.texture_def().extents;

            let renderbuffer = render_target.texture.gl_texture().unwrap().gl_raw_image().gl_renderbuffer_id().unwrap();
            gl_context.gl_bind_renderbuffer(gles20::RENDERBUFFER, renderbuffer)?;
            if renderbuffer != NONE_RENDERBUFFER {
                gl_context.gl_framebuffer_renderbuffer(gles20::FRAMEBUFFER, gles20::COLOR_ATTACHMENT0 + index as u32, gles20::RENDERBUFFER, renderbuffer)?;
            }

            if render_target.load_op == RafxLoadOp::Clear {
                let c = &render_target.clear_value.0;
                gl_context.gl_clear_color(c[0], c[1], c[2], c[3])?;
                clear_mask |= gles20::COLOR_BUFFER_BIT;
            }

            gl_context.gl_bind_renderbuffer(gles20::RENDERBUFFER, NONE_RENDERBUFFER)?;
        }

        if let Some(depth_target) = depth_target {
            let format = depth_target.texture.texture_def().format;
            if format.has_depth() {
                extents = depth_target.texture.texture_def().extents;

                let renderbuffer = depth_target.texture.gl_texture().unwrap().gl_raw_image().gl_renderbuffer_id().unwrap();
                gl_context.gl_bind_renderbuffer(gles20::RENDERBUFFER, renderbuffer)?;
                if renderbuffer != NONE_RENDERBUFFER {
                    gl_context.gl_framebuffer_renderbuffer(gles20::FRAMEBUFFER, gles20::DEPTH_ATTACHMENT, gles20::RENDERBUFFER, renderbuffer)?;
                }

                if depth_target.depth_load_op == RafxLoadOp::Clear {
                    gl_context.gl_clear_depthf(depth_target.clear_value.depth)?;
                    clear_mask |= gles20::DEPTH_BUFFER_BIT;
                }

                gl_context.gl_bind_renderbuffer(gles20::RENDERBUFFER, NONE_RENDERBUFFER)?;
            }

            if format.has_stencil() {
                extents = depth_target.texture.texture_def().extents;

                let renderbuffer = depth_target.texture.gl_texture().unwrap().gl_raw_image().gl_renderbuffer_id().unwrap();
                gl_context.gl_bind_renderbuffer(gles20::RENDERBUFFER, renderbuffer)?;
                if renderbuffer != NONE_RENDERBUFFER {
                    gl_context.gl_framebuffer_renderbuffer(gles20::FRAMEBUFFER, gles20::STENCIL_ATTACHMENT, gles20::RENDERBUFFER, renderbuffer)?;
                }

                if depth_target.stencil_load_op == RafxLoadOp::Clear {
                    gl_context.gl_clear_stencil(depth_target.clear_value.stencil as _)?;
                    clear_mask |= gles20::STENCIL_BUFFER_BIT;
                }

                gl_context.gl_bind_renderbuffer(gles20::RENDERBUFFER, NONE_RENDERBUFFER)?;
            }
        }

        if clear_mask != 0 {
            gl_context.gl_clear(clear_mask)?;
        }

        let result = gl_context.gl_check_framebuffer_status(gles20::FRAMEBUFFER)?;
        if result != gles20::FRAMEBUFFER_COMPLETE {
            log::error!("Incomplete framebuffer {}", result);
        }

        state.surface_size = Some(extents.to_2d());

        std::mem::drop(state);
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
        let mut state = self.command_pool_state.borrow_mut();
        assert!(state.is_started);

        let gl_context = self.queue.device_context().gl_context();
        let y_offset = state.surface_size.unwrap().height as f32 - y - height;

        gl_context.gl_viewport(x as _, y_offset as _, width as _, height as _)?;
        gl_context.gl_depth_rangef(depth_min, depth_max)
    }

    pub fn cmd_set_scissor(
        &self,
        mut x: u32,
        mut y: u32,
        mut width: u32,
        mut height: u32,
    ) -> RafxResult<()> {
        let mut state = self.command_pool_state.borrow_mut();
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
            Self::do_set_stencil_compare_ref_mask(gl_context, &info.gl_depth_stencil_state, state.stencil_reference_value)?;
        }

        Ok(())
    }

    // This logic is shared between cmd_set_stencil_reference_value and cmd_bind_pipeline
    fn do_set_stencil_compare_ref_mask(gl_context: &GlContext, state: &GlDepthStencilState, stencil_reference_value: u32) -> RafxResult<()> {
        if state.stencil_test_enable {
            gl_context.gl_stencil_func_separate(gles20::FRONT, state.front_stencil_compare_op, 0, !0)?;
            gl_context.gl_stencil_func_separate(gles20::BACK, state.back_stencil_compare_op, 0, !0)?;
        }

        Ok(())
    }

    pub fn cmd_bind_pipeline(
        &self,
        pipeline: &RafxPipelineGl,
    ) -> RafxResult<()> {
        let mut state = self.command_pool_state.borrow_mut();
        assert!(state.is_started);

        let pipeline_info = pipeline.gl_pipeline_info();
        state.current_gl_pipeline_info = Some(pipeline_info.clone());

        let gl_rasterizer_state = &pipeline_info.gl_rasterizer_state;
        let gl_depth_stencil_state = &pipeline_info.gl_depth_stencil_state;
        let gl_blend_state = &pipeline_info.gl_blend_state;

        let gl_context = self.queue.device_context().gl_context();
        gl_context.gl_use_program(pipeline.gl_program_id())?;

        let max_attribs = self.queue.device_context().device_info().max_vertex_attribute_count;
        for i in 0..max_attribs {
            gl_context.gl_disable_vertex_attrib_array(i);
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

            Self::do_set_stencil_compare_ref_mask(gl_context, gl_depth_stencil_state, state.stencil_reference_value)?;

            gl_context.gl_stencil_op_separate(
                gles20::FRONT,
                gl_depth_stencil_state.front_stencil_fail_op,
                gl_depth_stencil_state.front_depth_fail_op,
                gl_depth_stencil_state.front_stencil_pass_op
            )?;

            gl_context.gl_stencil_op_separate(
                gles20::BACK,
                gl_depth_stencil_state.back_stencil_fail_op,
                gl_depth_stencil_state.back_depth_fail_op,
                gl_depth_stencil_state.back_stencil_pass_op
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
                gl_blend_state.dst_factor_alpha
            )?;

            gl_context.gl_blend_equation_separate(gl_blend_state.blend_op, gl_blend_state.blend_op_alpha)?;
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
        assert!(first_binding + bindings.len() as u32 <= self.queue.device_context().device_info().max_vertex_attribute_count);

        let gl_pipeline_info = state.current_gl_pipeline_info.as_ref().unwrap().clone();
        let gl_context = self.queue.device_context().gl_context();

        let mut binding_index = first_binding;
        for binding in bindings {
            let gl_buffer = binding.buffer.gl_buffer().unwrap();
            assert_eq!(gl_buffer.gl_target(), gles20::ARRAY_BUFFER);

            // Bind the vertex buffer
            gl_context.gl_bind_buffer(gl_buffer.gl_target(), gl_buffer.gl_buffer_id().unwrap());
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
                    byte_offset
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

        let gl_pipeline_info = state.current_gl_pipeline_info.as_ref().unwrap().clone();
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
            descriptor_set_handle.array_index()
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
        let previous = &state.bound_descriptor_sets[set_index as usize];
        let mut bind_count = 1;
        if let Some(previous) = previous {
            bind_count = previous.update_index + 1;
        }

        state.bound_descriptor_sets[set_index as usize] = Some(BoundDescriptorSet {
            root_signature: root_signature.clone(),
            data: data.clone(),
            array_index,
            update_index: bind_count
        });
    }

    // Call right before drawing, this just checks that the program is up-to-date with the latest
    // bound descriptor sets
    fn ensure_pipeline_bindings_up_to_date(
        gl_context: &GlContext,
        state: &CommandPoolGlStateInner,
    ) -> RafxResult<()> {
        let pipeline = state.current_gl_pipeline_info.as_ref().unwrap();
        let mut last_descriptor_updates = pipeline.last_descriptor_updates.borrow_mut();

        for set_index in 0..MAX_DESCRIPTOR_SET_LAYOUTS {
            if let Some(bound_descriptor_set) = &state.bound_descriptor_sets[set_index] {
                if last_descriptor_updates[set_index] < bound_descriptor_set.update_index {
                    if bound_descriptor_set.root_signature == pipeline.root_signature {
                        Self::do_bind_descriptor_set(
                            gl_context,
                            pipeline,
                            &*bound_descriptor_set.data.borrow(),
                            set_index as u32,
                            bound_descriptor_set.array_index
                        )?;

                        last_descriptor_updates[set_index] = bound_descriptor_set.update_index;
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
        for descriptor_index in &root_signature.inner.layouts[set_index as usize].descriptors {
            let descriptor = &root_signature.inner.descriptors[descriptor_index.0 as usize];

            match descriptor.resource_type {
                RafxResourceType::BUFFER | RafxResourceType::BUFFER_READ_WRITE => {
                    let data_offset = descriptor.descriptor_data_offset_in_set.unwrap();
                    for i in 0..descriptor.element_count {
                        let buffer_state = data.buffer_states[(data_offset + i) as usize].as_ref().unwrap();

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
                },
                RafxResourceType::TEXTURE | RafxResourceType::TEXTURE_READ_WRITE => {
                    unimplemented!()
                },
                RafxResourceType::UNIFORM_BUFFER => {
                    let data_offset = descriptor.descriptor_data_offset_in_set.unwrap();
                    for i in 0..descriptor.element_count {
                        let buffer_state = data.buffer_states[(data_offset + i) as usize].as_ref().unwrap();

                        let base_offset = buffer_state.offset;
                        let data = unsafe {
                            buffer_state.buffer_contents.as_ref().unwrap().as_slice()
                        };

                        let uniform_reflection_data = root_signature.uniform_reflection_data();
                        let uniform_index = root_signature.uniform_index(descriptor.descriptor_index);

                        if let Some(uniform_index) = uniform_index {
                            let fields = uniform_reflection_data.uniform_fields(uniform_index);
                            for field in fields {
                                if let Some(location) = pipeline_info.uniform_member_location(field.field_index) {
                                    let field_offset = field.offset + base_offset as u32;
                                    assert!(field_offset + field.element_count <= data.len() as u32);
                                    unsafe {
                                        let data_ref = &*data.as_ptr().add(field_offset as usize);
                                        gl_type_util::set_uniform(gl_context, location, data_ref, field.ty, field.element_count)?;
                                    }
                                }
                            }
                        }
                    }
                },
                _ => unimplemented!("Unrecognized descriptor type in do_bind_descriptor_set")
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
        let pipeline_info = state.current_gl_pipeline_info.as_ref().unwrap();
        Self::ensure_pipeline_bindings_up_to_date(gl_context, &*state)?;

        gl_context.gl_draw_arrays(pipeline_info.gl_topology, first_vertex as _, vertex_count as _)?;

        Ok(())
    }

    pub fn cmd_draw_instanced(
        &self,
        vertex_count: u32,
        first_vertex: u32,
        instance_count: u32,
        first_instance: u32,
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
        let pipeline_info = state.current_gl_pipeline_info.as_ref().unwrap();
        Self::ensure_pipeline_bindings_up_to_date(gl_context, &*state)?;

        if vertex_offset > 0 {
            unimplemented!("GL ES 2.0 does not support vertex offsets during glDrawElements");
        }

        let offset = first_index * (std::mem::size_of::<gles20::types::GLushort>() as u32) + state.index_buffer_byte_offset;
        gl_context.gl_draw_elements(pipeline_info.gl_topology, index_count as _, gles20::UNSIGNED_SHORT, offset)
    }

    pub fn cmd_draw_indexed_instanced(
        &self,
        index_count: u32,
        first_index: u32,
        instance_count: u32,
        first_instance: u32,
        vertex_offset: i32,
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
        unimplemented!();
        // let mut inner = self.inner.borrow_mut();
        // let blit_encoder = inner.blit_encoder.as_ref();
        // let blit_encoder = match blit_encoder {
        //     Some(x) => x,
        //     None => {
        //         let result: RafxResult<&gl_rs::BlitCommandEncoderRef> =
        //             objc::rc::autoreleasepool(|| {
        //                 Self::do_end_current_encoders(&self.queue, &mut *inner, false)?;
        //                 let encoder = inner
        //                     .command_buffer
        //                     .as_ref()
        //                     .unwrap()
        //                     .new_blit_command_encoder();
        //                 inner.blit_encoder = Some(encoder.to_owned());
        //                 Ok(inner.blit_encoder.as_ref().unwrap().as_ref())
        //             });
        //         result?
        //     }
        // };
        //
        // blit_encoder.copy_from_buffer(
        //     src_buffer.gl_buffer(),
        //     src_offset as _,
        //     dst_buffer.gl_buffer(),
        //     dst_offset as _,
        //     size as _,
        // );
        // Ok(())
    }

    pub fn cmd_copy_buffer_to_texture(
        &self,
        src_buffer: &RafxBufferGl,
        dst_texture: &RafxTextureGl,
        params: &RafxCmdCopyBufferToTextureParams,
    ) -> RafxResult<()> {
        unimplemented!();
        // let mut inner = self.inner.borrow_mut();
        // let blit_encoder = inner.blit_encoder.as_ref();
        // let blit_encoder = match blit_encoder {
        //     Some(x) => x,
        //     None => {
        //         let result: RafxResult<&gl_rs::BlitCommandEncoderRef> =
        //             objc::rc::autoreleasepool(|| {
        //                 Self::do_end_current_encoders(&self.queue, &mut *inner, false)?;
        //                 let encoder = inner
        //                     .command_buffer
        //                     .as_ref()
        //                     .unwrap()
        //                     .new_blit_command_encoder();
        //                 inner.blit_encoder = Some(encoder.to_owned());
        //                 Ok(inner.blit_encoder.as_ref().unwrap().as_ref())
        //             });
        //         result?
        //     }
        // };
        //
        // let texture_def = dst_texture.texture_def();
        // let width = 1.max(texture_def.extents.width >> params.mip_level);
        // let height = 1.max(texture_def.extents.height >> params.mip_level);
        // let depth = 1.max(texture_def.extents.depth >> params.mip_level);
        //
        // // For a compressed format, sourceBytesPerRow is the number of bytes from the start of one row of blocks to the start of the next row of blocks.
        // let format = texture_def.format;
        // let block_size_in_bytes = format.block_or_pixel_size_in_bytes();
        // let block_width_in_pixels = format.block_width_in_pixels();
        // let texture_width_in_blocks =
        //     rafx_base::memory::round_size_up_to_alignment_u32(width, block_width_in_pixels)
        //         / block_width_in_pixels;
        //
        // let device_info = self.queue.device_context().device_info();
        // let texture_alignment = device_info.upload_buffer_texture_alignment;
        // let row_alignment = device_info.upload_buffer_texture_row_alignment;
        //
        // let source_bytes_per_row = rafx_base::memory::round_size_up_to_alignment_u32(
        //     texture_width_in_blocks * block_size_in_bytes,
        //     row_alignment,
        // );
        // let source_bytes_per_image = rafx_base::memory::round_size_up_to_alignment_u32(
        //     height * source_bytes_per_row,
        //     texture_alignment,
        // );
        //
        // let source_size = MTLSize {
        //     width: width as _,
        //     height: height as _,
        //     depth: depth as _,
        // };
        //
        // blit_encoder.copy_from_buffer_to_texture(
        //     src_buffer.gl_buffer(),
        //     params.buffer_offset as _,
        //     source_bytes_per_row as _,
        //     source_bytes_per_image as _,
        //     source_size,
        //     dst_texture.gl_texture(),
        //     params.array_layer as _,
        //     params.mip_level as _,
        //     MTLOrigin { x: 0, y: 0, z: 0 },
        //     MTLBlitOption::empty(),
        // );
        // Ok(())
    }
}
