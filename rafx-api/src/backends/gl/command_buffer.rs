use crate::gl::{DescriptorSetArrayData, RafxBufferGl, RafxCommandPoolGl, RafxDescriptorSetArrayGl, RafxDescriptorSetHandleGl, RafxPipelineGl, RafxQueueGl, RafxRootSignatureGl, RafxTextureGl, CommandPoolGlState, NONE_RENDERBUFFER};
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
    // pub fn gl_command_buffer(&self) -> Option<&gl_rs::CommandBufferRef> {
    //     use foreign_types_shared::ForeignType;
    //     use foreign_types_shared::ForeignTypeRef;
    //     let ptr = self
    //         .inner
    //         .borrow()
    //         .command_buffer
    //         .as_ref()
    //         .map(|x| x.as_ptr());
    //     ptr.map(|x| unsafe { gl_rs::CommandBufferRef::from_ptr(x) })
    // }

    // pub(crate) fn clear_command_buffer(&self) {
    //     self.inner.borrow_mut().command_buffer = None;
    // }

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
        state.is_started = false;
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
                    gl_context.gl_clear_stencil(depth_target.clear_value.stencil)?;
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
        // self.inner
        //     .borrow()
        //     .render_encoder
        //     .as_ref()
        //     .unwrap()
        //     .set_stencil_reference_value(value);
        // Ok(())
    }

    pub fn cmd_bind_pipeline(
        &self,
        pipeline: &RafxPipelineGl,
    ) -> RafxResult<()> {
        unimplemented!();
        // objc::rc::autoreleasepool(|| {
        //     let mut inner = self.inner.borrow_mut();
        //     let last_pipeline_type = inner.last_pipeline_type;
        //     inner.last_pipeline_type = Some(pipeline.pipeline_type());
        //
        //     let barrier_required = last_pipeline_type != Some(pipeline.pipeline_type());
        //
        //     match pipeline.pipeline_type() {
        //         RafxPipelineType::Graphics => {
        //             let render_encoder = inner.render_encoder.as_ref().unwrap();
        //             let render_encoder_info = pipeline.render_encoder_info.as_ref().unwrap();
        //             render_encoder
        //                 .set_render_pipeline_state(pipeline.gl_render_pipeline().unwrap());
        //             render_encoder.set_cull_mode(render_encoder_info.mtl_cull_mode);
        //             render_encoder
        //                 .set_front_facing_winding(render_encoder_info.mtl_front_facing_winding);
        //             render_encoder
        //                 .set_triangle_fill_mode(render_encoder_info.mtl_triangle_fill_mode);
        //             render_encoder.set_depth_bias(
        //                 render_encoder_info.mtl_depth_bias,
        //                 render_encoder_info.mtl_depth_bias_slope_scaled,
        //                 0.0,
        //             );
        //             render_encoder.set_depth_clip_mode(render_encoder_info.mtl_depth_clip_mode);
        //             if let Some(mtl_depth_stencil_state) =
        //                 &render_encoder_info.mtl_depth_stencil_state
        //             {
        //                 render_encoder.set_depth_stencil_state(mtl_depth_stencil_state);
        //             }
        //
        //             inner.primitive_type = render_encoder_info.mtl_primitive_type;
        //             self.flush_render_targets_to_make_readable(&mut *inner);
        //         }
        //         RafxPipelineType::Compute => {
        //             if !inner.compute_encoder.is_some() {
        //                 Self::do_end_current_encoders(&self.queue, &mut *inner, barrier_required)?;
        //
        //                 let compute_encoder = inner
        //                     .command_buffer
        //                     .as_ref()
        //                     .unwrap()
        //                     .new_compute_command_encoder();
        //                 inner.compute_encoder = Some(compute_encoder.to_owned());
        //             }
        //
        //             let compute_encoder_info = pipeline.compute_encoder_info.as_ref().unwrap();
        //             let compute_threads_per_group = compute_encoder_info.compute_threads_per_group;
        //             inner.compute_threads_per_group_x = compute_threads_per_group[0];
        //             inner.compute_threads_per_group_y = compute_threads_per_group[1];
        //             inner.compute_threads_per_group_z = compute_threads_per_group[2];
        //
        //             inner
        //                 .compute_encoder
        //                 .as_ref()
        //                 .unwrap()
        //                 .set_compute_pipeline_state(pipeline.gl_compute_pipeline().unwrap());
        //             self.flush_render_targets_to_make_readable(&mut *inner);
        //         }
        //     }
        //
        //     Ok(())
        // })
    }

    pub fn cmd_bind_vertex_buffers(
        &self,
        first_binding: u32,
        bindings: &[RafxVertexBufferBinding],
    ) -> RafxResult<()> {
        unimplemented!();
        // let inner = self.inner.borrow();
        // let render_encoder = inner.render_encoder.as_ref().unwrap();
        //
        // let mut binding_index = first_binding;
        // for binding in bindings {
        //     render_encoder.set_vertex_buffer(
        //         super::util::vertex_buffer_adjusted_buffer_index(binding_index),
        //         Some(binding.buffer.gl_buffer().unwrap().gl_buffer()),
        //         binding.byte_offset as _,
        //     );
        //
        //     binding_index += 1;
        // }
        //
        // Ok(())
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
        unimplemented!();
        // let inner = self.inner.borrow();
        // inner.render_encoder.as_ref().unwrap().draw_primitives(
        //     inner.primitive_type,
        //     first_vertex as _,
        //     vertex_count as _,
        // );
        // Ok(())
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
        group_count_x: u32,
        group_count_y: u32,
        group_count_z: u32,
    ) -> RafxResult<()> {
        unimplemented!();
        // let inner = self.inner.borrow();
        // self.wait_for_barriers(&*inner)?;
        // let thread_per_group = MTLSize {
        //     width: inner.compute_threads_per_group_x as _,
        //     height: inner.compute_threads_per_group_y as _,
        //     depth: inner.compute_threads_per_group_z as _,
        // };
        //
        // let group_count = MTLSize {
        //     width: group_count_x as _,
        //     height: group_count_y as _,
        //     depth: group_count_z as _,
        // };
        //
        // inner
        //     .compute_encoder
        //     .as_ref()
        //     .unwrap()
        //     .dispatch_thread_groups(group_count, thread_per_group);
        // Ok(())
    }

    pub fn cmd_resource_barrier(
        &self,
        buffer_barriers: &[RafxBufferBarrier],
        texture_barriers: &[RafxTextureBarrier],
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
