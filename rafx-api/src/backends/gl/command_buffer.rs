use crate::gl::{DescriptorSetArrayData, RafxBufferGl, RafxCommandPoolGl, RafxDescriptorSetArrayGl, RafxDescriptorSetHandleGl, RafxPipelineGl, RafxQueueGl, RafxRootSignatureGl, RafxTextureGl, CommandPoolGlState, NONE_RENDERBUFFER, NONE_VERTEX_ARRAY_OBJECT};
use crate::{RafxBufferBarrier, RafxCmdCopyBufferToTextureParams, RafxColorRenderTargetBinding, RafxCommandBufferDef, RafxDepthStencilRenderTargetBinding, RafxExtents3D, RafxIndexBufferBinding, RafxIndexType, RafxLoadOp, RafxPipelineType, RafxResourceState, RafxResult, RafxTextureBarrier, RafxVertexBufferBinding, RafxExtents2D};
use fnv::FnvHashSet;
// use gl_rs::{
//     MTLBlitOption, MTLIndexType, MTLOrigin, MTLPrimitiveType, MTLRenderStages, MTLResourceUsage,
//     MTLScissorRect, MTLSize, MTLViewport,
// };
use rafx_base::trust_cell::TrustCell;

use crate::gl::gles20;

// Mutable state stored in a lock. (Hopefully we can optimize away the lock later)
// #[derive(Debug)]
// pub struct RafxCommandBufferGlInner {
//     // render_targets_to_make_readable: FnvHashSet<RafxTextureGl>,
//     // command_buffer: Option<gl_rs::CommandBuffer>,
//     // render_encoder: Option<gl_rs::RenderCommandEncoder>,
//     // compute_encoder: Option<gl_rs::ComputeCommandEncoder>,
//     // blit_encoder: Option<gl_rs::BlitCommandEncoder>,
//     // current_index_buffer: Option<gl_rs::Buffer>,
//     // current_index_buffer_byte_offset: u64,
//     // current_index_buffer_type: MTLIndexType,
//     // current_index_buffer_stride: u32,
//     // last_pipeline_type: Option<RafxPipelineType>,
//     // primitive_type: MTLPrimitiveType,
//     // current_render_targets_width: u32,
//     // current_render_targets_height: u32,
//     // compute_threads_per_group_x: u32,
//     // compute_threads_per_group_y: u32,
//     // compute_threads_per_group_z: u32,
// }

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

        let vao = self.queue.gl_vertex_buffer_array_object();
        self.queue.device_context().gl_context().gl_bind_vertex_array(vao);

        Ok(())
    }

    pub fn end(&self) -> RafxResult<()> {
        let mut state = self.command_pool_state.borrow_mut();
        assert!(state.is_started);
        assert!(state.surface_size.is_none());

        state.is_started = false;
        state.current_gl_pipeline_info = None;
        for offset in &mut state.vertex_buffer_begin_offset {
            *offset = 0;
        }

        self.queue.device_context().gl_context().gl_bind_vertex_array(NONE_VERTEX_ARRAY_OBJECT);

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
        unimplemented!();
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

            gl_context.gl_stencil_func_separate(gles20::FRONT, gl_depth_stencil_state.front_stencil_compare_op, 0, !0)?;
            gl_context.gl_stencil_op_separate(
                gles20::FRONT,
                gl_depth_stencil_state.front_stencil_fail_op,
                gl_depth_stencil_state.front_depth_fail_op,
                gl_depth_stencil_state.front_stencil_pass_op
            )?;

            gl_context.gl_stencil_func_separate(gles20::BACK, gl_depth_stencil_state.back_stencil_compare_op, 0, !0)?;
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
            state.vertex_buffer_begin_offset[binding_index as usize] = binding.byte_offset as u32;

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
                dbg!(attribute, byte_offset);

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
        unimplemented!();
        // let mut inner = self.inner.borrow_mut();
        // inner.current_index_buffer = Some(
        //     binding
        //         .buffer
        //         .gl_buffer()
        //         .unwrap()
        //         .gl_buffer()
        //         .to_owned(),
        // );
        // inner.current_index_buffer_byte_offset = binding.byte_offset;
        // inner.current_index_buffer_type = binding.index_type.into();
        // inner.current_index_buffer_stride = match binding.index_type {
        //     RafxIndexType::Uint32 => std::mem::size_of::<u32>() as _,
        //     RafxIndexType::Uint16 => std::mem::size_of::<u16>() as _,
        // };
        // Ok(())
    }

    pub fn cmd_bind_descriptor_set(
        &self,
        descriptor_set_array: &RafxDescriptorSetArrayGl,
        index: u32,
    ) -> RafxResult<()> {
        unimplemented!();
        // let (buffer, offset) = descriptor_set_array
        //     .gl_argument_buffer_and_offset(index)
        //     .unwrap();
        // self.do_bind_descriptor_set(
        //     &*self.inner.borrow(),
        //     descriptor_set_array
        //         .root_signature()
        //         .gl_root_signature()
        //         .unwrap(),
        //     descriptor_set_array.set_index(),
        //     buffer,
        //     offset,
        //     index,
        //     descriptor_set_array.argument_buffer_data().unwrap(),
        // )
    }

    pub fn cmd_bind_descriptor_set_handle(
        &self,
        root_signature: &RafxRootSignatureGl,
        set_index: u32,
        descriptor_set_handle: &RafxDescriptorSetHandleGl,
    ) -> RafxResult<()> {
        unimplemented!();
        // let buffer = descriptor_set_handle.gl_buffer();
        // let offset = descriptor_set_handle.offset();
        // let array_index = descriptor_set_handle.array_index();
        // self.do_bind_descriptor_set(
        //     &*self.inner.borrow(),
        //     root_signature,
        //     set_index,
        //     buffer,
        //     offset,
        //     array_index,
        //     descriptor_set_handle.argument_buffer_data(),
        // )
    }

    // fn do_bind_descriptor_set(
    //     &self,
    //     inner: &RafxCommandBufferGlInner,
    //     root_signature: &RafxRootSignatureGl,
    //     set_index: u32,
    //     //argument_buffer: &gl_rs::BufferRef,
    //     argument_buffer_offset: u32,
    //     array_index: u32,
    //     argument_buffer_data: &ArgumentBufferData,
    // ) -> RafxResult<()> {
    //     unimplemented!();
    //     // match root_signature.pipeline_type() {
    //     //     RafxPipelineType::Graphics => {
    //     //         let render_encoder = inner
    //     //             .render_encoder
    //     //             .as_ref()
    //     //             .ok_or("Must begin render pass before binding graphics descriptor sets")?;
    //     //         render_encoder.set_vertex_buffer(
    //     //             set_index as _,
    //     //             Some(argument_buffer),
    //     //             argument_buffer_offset as _,
    //     //         );
    //     //         render_encoder.set_fragment_buffer(
    //     //             set_index as _,
    //     //             Some(argument_buffer),
    //     //             argument_buffer_offset as _,
    //     //         );
    //     //         argument_buffer_data
    //     //             .make_resources_resident_render_encoder(array_index, render_encoder);
    //     //     }
    //     //     RafxPipelineType::Compute => {
    //     //         let compute_encoder = inner
    //     //             .compute_encoder
    //     //             .as_ref()
    //     //             .ok_or("Must bind compute pipeline before binding compute descriptor sets")?;
    //     //         compute_encoder.set_buffer(
    //     //             set_index as _,
    //     //             Some(argument_buffer),
    //     //             argument_buffer_offset as _,
    //     //         );
    //     //         argument_buffer_data
    //     //             .make_resources_resident_compute_encoder(array_index, compute_encoder);
    //     //     }
    //     // }
    //     //
    //     // Ok(())
    // }

    pub fn cmd_draw(
        &self,
        vertex_count: u32,
        first_vertex: u32,
    ) -> RafxResult<()> {
        let mut state = self.command_pool_state.borrow_mut();
        assert!(state.is_started);

        let gl_context = self.queue.device_context().gl_context();
        let pipeline_info = state.current_gl_pipeline_info.as_ref().unwrap();
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
        unimplemented!();
        // let features = self.queue.device_context().gl_features();
        // if !features.supports_base_vertex_instance_drawing {
        //     assert_eq!(first_instance, 0);
        // }
        //
        // let inner = self.inner.borrow();
        //
        // if first_instance == 0 {
        //     inner
        //         .render_encoder
        //         .as_ref()
        //         .unwrap()
        //         .draw_primitives_instanced(
        //             inner.primitive_type,
        //             first_vertex as _,
        //             vertex_count as _,
        //             instance_count as _,
        //         );
        // } else {
        //     inner
        //         .render_encoder
        //         .as_ref()
        //         .unwrap()
        //         .draw_primitives_instanced_base_instance(
        //             inner.primitive_type,
        //             first_vertex as _,
        //             vertex_count as _,
        //             instance_count as _,
        //             first_instance as _,
        //         );
        // }
        // Ok(())
    }

    pub fn cmd_draw_indexed(
        &self,
        index_count: u32,
        first_index: u32,
        vertex_offset: i32,
    ) -> RafxResult<()> {
        unimplemented!();
        // let features = self.queue.device_context().gl_features();
        // if !features.supports_base_vertex_instance_drawing {
        //     assert_eq!(vertex_offset, 0);
        // }
        //
        // let inner = self.inner.borrow();
        // let stride = inner.current_index_buffer_stride;
        // if vertex_offset == 0 {
        //     inner
        //         .render_encoder
        //         .as_ref()
        //         .unwrap()
        //         .draw_indexed_primitives(
        //             inner.primitive_type,
        //             index_count as _,
        //             inner.current_index_buffer_type,
        //             inner.current_index_buffer.as_ref().unwrap(),
        //             ((stride * first_index) as u64 + inner.current_index_buffer_byte_offset) as _,
        //         );
        // } else {
        //     inner
        //         .render_encoder
        //         .as_ref()
        //         .unwrap()
        //         .draw_indexed_primitives_instanced_base_instance(
        //             inner.primitive_type,
        //             index_count as _,
        //             inner.current_index_buffer_type,
        //             inner.current_index_buffer.as_ref().unwrap(),
        //             ((stride * first_index) as u64 + inner.current_index_buffer_byte_offset) as _,
        //             1,
        //             vertex_offset as _,
        //             0,
        //         );
        // }
        //
        // Ok(())
    }

    pub fn cmd_draw_indexed_instanced(
        &self,
        index_count: u32,
        first_index: u32,
        instance_count: u32,
        first_instance: u32,
        vertex_offset: i32,
    ) -> RafxResult<()> {
        unimplemented!();
        // let features = self.queue.device_context().gl_features();
        // if !features.supports_base_vertex_instance_drawing {
        //     assert_eq!(vertex_offset, 0);
        //     assert_eq!(first_instance, 0);
        // }
        //
        // let inner = self.inner.borrow();
        // let stride = inner.current_index_buffer_stride;
        // if vertex_offset == 0 && first_instance == 0 {
        //     inner
        //         .render_encoder
        //         .as_ref()
        //         .unwrap()
        //         .draw_indexed_primitives_instanced(
        //             inner.primitive_type,
        //             index_count as _,
        //             inner.current_index_buffer_type,
        //             inner.current_index_buffer.as_ref().unwrap(),
        //             ((stride * first_index) as u64 + inner.current_index_buffer_byte_offset) as _,
        //             instance_count as _,
        //         );
        // } else {
        //     inner
        //         .render_encoder
        //         .as_ref()
        //         .unwrap()
        //         .draw_indexed_primitives_instanced_base_instance(
        //             inner.primitive_type,
        //             index_count as _,
        //             inner.current_index_buffer_type,
        //             inner.current_index_buffer.as_ref().unwrap(),
        //             ((stride * first_index) as u64 + inner.current_index_buffer_byte_offset) as _,
        //             instance_count as _,
        //             vertex_offset as _,
        //             first_instance as _,
        //         );
        // }
        //
        // Ok(())
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
