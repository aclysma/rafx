use crate::dx12::{
    Dx12DescriptorId, RafxBufferDx12, RafxCommandPoolDx12, RafxDescriptorSetArrayDx12,
    RafxDescriptorSetHandleDx12, RafxPipelineDx12, RafxQueueDx12, RafxRootSignatureDx12,
    RafxTextureDx12,
};
use crate::{
    RafxBarrierQueueTransition, RafxBufferBarrier, RafxCmdCopyBufferToBufferParams,
    RafxCmdCopyBufferToTextureParams, RafxCmdCopyTextureToTextureParams,
    RafxColorRenderTargetBinding, RafxCommandBufferDef, RafxDepthStencilRenderTargetBinding,
    RafxDescriptorIndex, RafxExtents3D, RafxIndexBufferBinding, RafxIndexType, RafxLoadOp,
    RafxMemoryUsage, RafxPipelineType, RafxQueueType, RafxResourceState, RafxResourceType,
    RafxResult, RafxTextureBarrier, RafxVertexBufferBinding,
};
use rafx_base::trust_cell::TrustCell;
use std::mem::ManuallyDrop;
use windows::core::Interface;
use windows::Win32::Graphics::Direct3D12::ID3D12RootSignature;

use super::d3d12;

// Mutable state stored in a lock. (Hopefully we can optimize away the lock later)
#[derive(Debug)]
pub struct RafxCommandBufferDx12Inner {
    //command_list_type: d3d12::D3D12_COMMAND_LIST_TYPE,
    command_list_base: d3d12::ID3D12CommandList,
    command_list: d3d12::ID3D12GraphicsCommandList6,
    command_allocator: d3d12::ID3D12CommandAllocator,
    bound_root_signature: Option<d3d12::ID3D12RootSignature>,

    vertex_buffer_strides: [u32; crate::MAX_VERTEX_INPUT_BINDINGS],
    _debug_names_enabled: bool,
}

unsafe impl Send for RafxCommandBufferDx12Inner {}
unsafe impl Sync for RafxCommandBufferDx12Inner {}

#[derive(Debug)]
pub struct RafxCommandBufferDx12 {
    queue: RafxQueueDx12,
    inner: TrustCell<RafxCommandBufferDx12Inner>,
}

impl RafxCommandBufferDx12 {
    pub fn dx12_command_list(&self) -> d3d12::ID3D12CommandList {
        self.inner.borrow().command_list_base.clone()
    }

    pub fn dx12_graphics_command_list(&self) -> d3d12::ID3D12GraphicsCommandList6 {
        self.inner.borrow().command_list.clone()
    }

    pub fn queue(&self) -> &RafxQueueDx12 {
        &self.queue
    }

    fn reset_root_signature(
        inner: &mut RafxCommandBufferDx12Inner,
        pipeline_type: RafxPipelineType,
        root_signature: &ID3D12RootSignature,
    ) -> RafxResult<()> {
        if let Some(bound_root_signature) = &inner.bound_root_signature {
            if bound_root_signature == root_signature {
                return Ok(());
            }
        }

        inner.bound_root_signature = Some(root_signature.clone());
        unsafe {
            match pipeline_type {
                RafxPipelineType::Graphics => {
                    inner.command_list.SetGraphicsRootSignature(root_signature)
                }
                RafxPipelineType::Compute => {
                    inner.command_list.SetComputeRootSignature(root_signature)
                }
            }
        }
        //TODO: Clear bound descriptor sets

        Ok(())
    }

    pub fn new(
        command_pool: &RafxCommandPoolDx12,
        _command_buffer_def: &RafxCommandBufferDef,
    ) -> RafxResult<RafxCommandBufferDx12> {
        //TODO: Special handling for copy?
        let command_list_type = command_pool.command_list_type();
        let command_list = unsafe {
            let command_list: d3d12::ID3D12GraphicsCommandList6 = command_pool
                .queue()
                .device_context()
                .d3d12_device()
                .CreateCommandList(0, command_list_type, command_pool.command_allocator(), None)?;
            command_list.Close()?;
            command_list
        };

        let command_list_base = command_list.clone().cast().unwrap();

        let inner = RafxCommandBufferDx12Inner {
            command_list,
            command_list_base,
            command_allocator: command_pool.command_allocator().clone(),
            bound_root_signature: None,
            vertex_buffer_strides: [0; crate::MAX_VERTEX_INPUT_BINDINGS],
            _debug_names_enabled: command_pool
                .device_context()
                .device_info()
                .debug_names_enabled,
        };

        Ok(RafxCommandBufferDx12 {
            queue: command_pool.queue().clone(),
            inner: TrustCell::new(inner),
        })
    }

    pub fn begin(&self) -> RafxResult<()> {
        let mut inner = self.inner.borrow_mut();

        unsafe {
            inner.command_list.Reset(&inner.command_allocator, None)?;
        }

        if self.queue.queue_type() != RafxQueueType::Transfer {
            let cbv_srv_uav_heap = self
                .queue
                .device_context()
                .inner
                .heaps
                .gpu_cbv_srv_uav_heap
                .dx12_heap();
            let sampler_heap = self
                .queue
                .device_context()
                .inner
                .heaps
                .gpu_sampler_heap
                .dx12_heap();
            unsafe {
                inner
                    .command_list
                    .SetDescriptorHeaps(&[cbv_srv_uav_heap, sampler_heap]);
            }
        }

        //TODO: Set heaps

        inner.bound_root_signature = None;

        Ok(())
    }

    pub fn end(&self) -> RafxResult<()> {
        unsafe {
            self.inner.borrow().command_list.Close()?;
        }

        Ok(())
    }

    pub fn return_to_pool(&self) -> RafxResult<()> {
        //TODO: Not sure if this is needed/correct?
        let inner = self.inner.borrow_mut();
        unsafe {
            inner.command_list.Reset(&inner.command_allocator, None)?;
        }

        Ok(())
    }

    pub fn cmd_begin_render_pass(
        &self,
        color_targets: &[RafxColorRenderTargetBinding],
        depth_target: Option<RafxDepthStencilRenderTargetBinding>,
    ) -> RafxResult<()> {
        let inner = self.inner.borrow_mut();

        let mut extents = RafxExtents3D::default();

        //bind render targets

        let mut rtvs = Vec::with_capacity(color_targets.len());
        for color_target in color_targets {
            extents = color_target.texture.texture_def().extents;
            if color_target.array_slice.is_none() && color_target.mip_slice.is_none() {
                rtvs.push(
                    color_target
                        .texture
                        .dx12_texture()
                        .unwrap()
                        .rtv_handle()
                        .unwrap(),
                );
            } else {
                rtvs.push(
                    color_target
                        .texture
                        .dx12_texture()
                        .unwrap()
                        .rtv_slice_handle(
                            color_target.mip_slice.unwrap_or(0) as u32,
                            color_target.array_slice.unwrap_or(0) as u32,
                        )
                        .unwrap(),
                );
            }
        }

        // OMSetRenderTargets needs a pointer to a descriptor handle. It's easy to accidentally have
        // pointer to a tempoary stack if we're not careful
        let mut dsv_value = d3d12::D3D12_CPU_DESCRIPTOR_HANDLE::default();
        let mut dsv_ptr = None;
        if let Some(depth_target) = &depth_target {
            extents = depth_target.texture.texture_def().extents;
            if depth_target.mip_slice.is_none() && depth_target.array_slice.is_none() {
                dsv_value = depth_target
                    .texture
                    .dx12_texture()
                    .unwrap()
                    .dsv_handle()
                    .unwrap();
            } else {
                dsv_value = depth_target
                    .texture
                    .dx12_texture()
                    .unwrap()
                    .dsv_slice_handle(
                        depth_target.mip_slice.unwrap_or(0) as u32,
                        depth_target.array_slice.unwrap_or(0) as u32,
                    )
                    .unwrap();
            }

            dsv_ptr = Some(&dsv_value as *const d3d12::D3D12_CPU_DESCRIPTOR_HANDLE);
        }

        let cmd_list = &inner.command_list;
        unsafe {
            cmd_list.OMSetRenderTargets(rtvs.len() as u32, Some(rtvs.as_ptr()), false, dsv_ptr);
        }

        for (i, color_target) in color_targets.iter().enumerate() {
            if color_target.load_op == RafxLoadOp::Clear {
                unsafe {
                    cmd_list.ClearRenderTargetView(
                        rtvs[i],
                        color_target.clear_value.0.as_ptr(),
                        &[],
                    );
                }
            }
        }

        if let Some(depth_target) = &depth_target {
            if depth_target.depth_load_op == RafxLoadOp::Clear
                || depth_target.stencil_load_op == RafxLoadOp::Clear
            {
                let mut flags = d3d12::D3D12_CLEAR_FLAGS::default();
                if depth_target.depth_load_op == RafxLoadOp::Clear {
                    flags |= d3d12::D3D12_CLEAR_FLAG_DEPTH;
                }
                if depth_target.stencil_load_op == RafxLoadOp::Clear {
                    flags |= d3d12::D3D12_CLEAR_FLAG_STENCIL;
                }
                unsafe {
                    cmd_list.ClearDepthStencilView(
                        dsv_value,
                        flags,
                        depth_target.clear_value.depth,
                        depth_target.clear_value.stencil as u8,
                        &[],
                    );
                }
            }
        }

        drop(inner);

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
        unsafe {
            self.inner
                .borrow()
                .command_list
                .RSSetViewports(&[d3d12::D3D12_VIEWPORT {
                    TopLeftX: x,
                    TopLeftY: y,
                    Width: width,
                    Height: height,
                    MinDepth: depth_min,
                    MaxDepth: depth_max,
                }]);
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
            self.inner.borrow().command_list.RSSetScissorRects(&[
                windows::Win32::Foundation::RECT {
                    left: x as i32,
                    top: y as i32,
                    right: (x + width) as i32,
                    bottom: (y + height) as i32,
                },
            ]);
        }
        Ok(())
    }

    pub fn cmd_set_stencil_reference_value(
        &self,
        _value: u32,
    ) -> RafxResult<()> {
        unimplemented!()
    }

    pub fn cmd_bind_pipeline(
        &self,
        pipeline: &RafxPipelineDx12,
    ) -> RafxResult<()> {
        let mut inner = self.inner.borrow_mut();

        inner.vertex_buffer_strides = *pipeline.vertex_buffer_strides();

        match pipeline.pipeline_type() {
            RafxPipelineType::Graphics => {
                Self::reset_root_signature(
                    &mut *inner,
                    pipeline.pipeline_type(),
                    pipeline
                        .root_signature()
                        .dx12_root_signature()
                        .unwrap()
                        .dx12_root_signature(),
                )?;
                let cmd_list = &inner.command_list;
                unsafe {
                    cmd_list.IASetPrimitiveTopology(pipeline.topology());
                    cmd_list.SetPipelineState(pipeline.pipeline());
                }
            }
            RafxPipelineType::Compute => {
                Self::reset_root_signature(
                    &mut *inner,
                    pipeline.pipeline_type(),
                    pipeline
                        .root_signature()
                        .dx12_root_signature()
                        .unwrap()
                        .dx12_root_signature(),
                )?;
                let cmd_list = &inner.command_list;
                unsafe {
                    cmd_list.SetPipelineState(pipeline.pipeline());
                }
            }
        }

        Ok(())
    }

    pub fn cmd_bind_vertex_buffers(
        &self,
        first_binding: u32,
        bindings: &[RafxVertexBufferBinding],
    ) -> RafxResult<()> {
        let inner = self.inner.borrow();

        let mut buffer_views = Vec::with_capacity(bindings.len());
        for (buffer_index, binding) in bindings.iter().enumerate() {
            buffer_views.push(d3d12::D3D12_VERTEX_BUFFER_VIEW {
                BufferLocation: binding.buffer.dx12_buffer().unwrap().gpu_address()
                    + binding.byte_offset,
                SizeInBytes: binding.buffer.dx12_buffer().unwrap().buffer_def().size as u32
                    - binding.byte_offset as u32,
                StrideInBytes: inner.vertex_buffer_strides[buffer_index],
            });
        }
        unsafe {
            inner
                .command_list
                .IASetVertexBuffers(first_binding, Some(&buffer_views));
        }

        Ok(())
    }

    pub fn cmd_bind_index_buffer(
        &self,
        binding: &RafxIndexBufferBinding,
    ) -> RafxResult<()> {
        let format = match binding.index_type {
            RafxIndexType::Uint16 => super::dxgi::Common::DXGI_FORMAT_R16_UINT,
            RafxIndexType::Uint32 => super::dxgi::Common::DXGI_FORMAT_R32_UINT,
        };

        let view = d3d12::D3D12_INDEX_BUFFER_VIEW {
            BufferLocation: binding.buffer.dx12_buffer().unwrap().gpu_address()
                + binding.byte_offset,
            SizeInBytes: (binding.buffer.buffer_def().size - binding.byte_offset) as u32,
            Format: format,
        };

        let inner = self.inner.borrow_mut();
        unsafe {
            inner.command_list.IASetIndexBuffer(Some(&view));
        }

        Ok(())
    }

    fn do_bind_descriptor_set(
        inner: &mut RafxCommandBufferDx12Inner,
        root_signature: &RafxRootSignatureDx12,
        cbv_srv_uav_descriptor: Option<(Dx12DescriptorId, u8)>,
        sampler_descriptor: Option<(Dx12DescriptorId, u8)>,
    ) -> RafxResult<()> {
        Self::reset_root_signature(
            &mut *inner,
            root_signature.pipeline_type(),
            root_signature.dx12_root_signature(),
        )?;

        if let Some(descriptor) = cbv_srv_uav_descriptor {
            let dx12_handle = root_signature
                .device_context()
                .inner
                .heaps
                .gpu_cbv_srv_uav_heap
                .id_to_gpu_handle(descriptor.0);
            unsafe {
                match root_signature.pipeline_type() {
                    RafxPipelineType::Graphics => inner
                        .command_list
                        .SetGraphicsRootDescriptorTable(descriptor.1 as u32, dx12_handle),
                    RafxPipelineType::Compute => inner
                        .command_list
                        .SetComputeRootDescriptorTable(descriptor.1 as u32, dx12_handle),
                }
            }
        }

        if let Some(descriptor) = sampler_descriptor {
            let dx12_handle = root_signature
                .device_context()
                .inner
                .heaps
                .gpu_sampler_heap
                .id_to_gpu_handle(descriptor.0);
            unsafe {
                match root_signature.pipeline_type() {
                    RafxPipelineType::Graphics => inner
                        .command_list
                        .SetGraphicsRootDescriptorTable(descriptor.1 as u32, dx12_handle),
                    RafxPipelineType::Compute => inner
                        .command_list
                        .SetComputeRootDescriptorTable(descriptor.1 as u32, dx12_handle),
                }
            }
        }

        Ok(())
    }

    pub fn cmd_bind_descriptor_set(
        &self,
        descriptor_set_array: &RafxDescriptorSetArrayDx12,
        index: u32,
    ) -> RafxResult<()> {
        let mut inner = self.inner.borrow_mut();
        let root_signature = descriptor_set_array.root_signature();
        Self::reset_root_signature(
            &mut *inner,
            root_signature.pipeline_type(),
            root_signature
                .dx12_root_signature()
                .unwrap()
                .dx12_root_signature(),
        )?;

        let mut cbv_srv_uav_descriptor = None;
        let mut sampler_descriptor = None;
        if let Some(x) = descriptor_set_array.cbv_srv_uav_table_info() {
            let id = Dx12DescriptorId(x.first_id.0 + x.stride * index);
            cbv_srv_uav_descriptor = Some((id, x.root_index));
        }

        if let Some(x) = descriptor_set_array.sampler_table_info() {
            let id = Dx12DescriptorId(x.first_id.0 + x.stride * index);
            sampler_descriptor = Some((id, x.root_index));
        }

        Self::do_bind_descriptor_set(
            &mut *inner,
            root_signature.dx12_root_signature().unwrap(),
            cbv_srv_uav_descriptor,
            sampler_descriptor,
        )
    }

    pub fn cmd_bind_descriptor_set_handle(
        &self,
        root_signature: &RafxRootSignatureDx12,
        _set_index: u32,
        descriptor_set_handle: &RafxDescriptorSetHandleDx12,
    ) -> RafxResult<()> {
        let mut inner = self.inner.borrow_mut();

        let cbv_srv_uav_descriptor = descriptor_set_handle
            .cbv_srv_uav_descriptor_id()
            .map(|x| (x, descriptor_set_handle.cbv_srv_uav_root_index()));
        let sampler_descriptor = descriptor_set_handle
            .sampler_descriptor_id()
            .map(|x| (x, descriptor_set_handle.sampler_root_index()));

        Self::do_bind_descriptor_set(
            &mut *inner,
            root_signature,
            cbv_srv_uav_descriptor,
            sampler_descriptor,
        )
    }

    pub fn cmd_bind_push_constant<T: Copy>(
        &self,
        root_signature: &RafxRootSignatureDx12,
        descriptor_index: RafxDescriptorIndex,
        data: &T,
    ) -> RafxResult<()> {
        let descriptor = root_signature.descriptor(descriptor_index).unwrap();
        assert_eq!(
            std::mem::size_of::<T>(),
            descriptor.push_constant_size as usize
        );

        let mut inner = self.inner.borrow_mut();
        Self::reset_root_signature(
            &mut *inner,
            root_signature.pipeline_type(),
            root_signature.dx12_root_signature(),
        )?;

        let num_32bit_values_to_set =
            rafx_base::memory::round_size_up_to_alignment_u32(descriptor.push_constant_size, 4) / 4;
        unsafe {
            match root_signature.pipeline_type() {
                RafxPipelineType::Graphics => {
                    inner.command_list.SetGraphicsRoot32BitConstants(
                        descriptor.root_param_index.unwrap(),
                        num_32bit_values_to_set,
                        rafx_base::memory::any_as_bytes(data).as_ptr() as _,
                        0,
                    );
                }
                RafxPipelineType::Compute => {
                    inner.command_list.SetComputeRoot32BitConstants(
                        descriptor.root_param_index.unwrap(),
                        num_32bit_values_to_set,
                        rafx_base::memory::any_as_bytes(data).as_ptr() as _,
                        0,
                    );
                }
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
        unsafe {
            inner
                .command_list
                .DrawInstanced(vertex_count, 1, first_vertex, 0);
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
        let inner = self.inner.borrow();
        unsafe {
            inner.command_list.DrawInstanced(
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
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
        let inner = self.inner.borrow();
        unsafe {
            inner
                .command_list
                .DrawIndexedInstanced(index_count, 1, first_index, vertex_offset, 0);
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
        let inner = self.inner.borrow();
        unsafe {
            inner.command_list.DrawIndexedInstanced(
                index_count,
                instance_count,
                first_index,
                vertex_offset,
                first_instance,
            );
        }

        Ok(())
    }

    pub fn cmd_draw_indirect(
        &self,
        indirect_buffer: &RafxBufferDx12,
        indirect_buffer_offset_in_bytes: u32,
        draw_count: u32,
    ) -> RafxResult<()> {
        let inner = self.inner.borrow();
        unsafe {
            inner.command_list.ExecuteIndirect(
                &self.queue.device_context().inner.indirect_command_signature,
                draw_count,
                indirect_buffer.dx12_resource(),
                indirect_buffer_offset_in_bytes as u64,
                None,
                0,
            );
        }

        Ok(())
    }

    pub fn cmd_draw_indexed_indirect(
        &self,
        indirect_buffer: &RafxBufferDx12,
        indirect_buffer_offset_in_bytes: u32,
        draw_count: u32,
    ) -> RafxResult<()> {
        let inner = self.inner.borrow();
        unsafe {
            inner.command_list.ExecuteIndirect(
                &self
                    .queue
                    .device_context()
                    .inner
                    .indirect_command_signature_indexed,
                draw_count,
                indirect_buffer.dx12_resource(),
                indirect_buffer_offset_in_bytes as u64,
                None,
                0,
            );
        }

        Ok(())
    }

    pub fn cmd_draw_mesh(
        &self,
        group_count_x: u32,
        group_count_y: u32,
        group_count_z: u32,
    ) -> RafxResult<()> {
        let inner = self.inner.borrow();
        unsafe {
            inner
                .command_list
                .DispatchMesh(group_count_x, group_count_y, group_count_z);
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
        unsafe {
            inner
                .command_list
                .Dispatch(group_count_x, group_count_y, group_count_z);
        }
        Ok(())
    }

    pub fn cmd_resource_barrier(
        &self,
        buffer_barriers: &[RafxBufferBarrier],
        texture_barriers: &[RafxTextureBarrier],
    ) -> RafxResult<()> {
        let mut barriers = Vec::default();

        for buffer_barrier in buffer_barriers {
            let buffer_def = buffer_barrier.buffer.buffer_def();
            let memory_usage = buffer_def.memory_usage;
            if memory_usage == RafxMemoryUsage::GpuOnly
                || memory_usage == RafxMemoryUsage::GpuToCpu
                || (memory_usage == RafxMemoryUsage::CpuToGpu
                    && buffer_def
                        .resource_type
                        .intersects(RafxResourceType::BUFFER_READ_WRITE))
            {
                if buffer_barrier
                    .src_state
                    .intersects(RafxResourceState::UNORDERED_ACCESS)
                    & buffer_barrier
                        .dst_state
                        .intersects(RafxResourceState::UNORDERED_ACCESS)
                {
                    let mut barrier = d3d12::D3D12_RESOURCE_BARRIER::default();
                    barrier.Type = d3d12::D3D12_RESOURCE_BARRIER_TYPE_UAV;
                    barrier.Flags = d3d12::D3D12_RESOURCE_BARRIER_FLAG_NONE;
                    barrier.Anonymous.UAV = ManuallyDrop::new(d3d12::D3D12_RESOURCE_UAV_BARRIER {
                        pResource: windows::core::ManuallyDrop::new(
                            buffer_barrier.buffer.dx12_buffer().unwrap().dx12_resource(),
                        ),
                    });
                    barriers.push(barrier);
                } else {
                    let mut barrier = d3d12::D3D12_RESOURCE_BARRIER::default();
                    barrier.Type = d3d12::D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
                    barrier.Flags = d3d12::D3D12_RESOURCE_BARRIER_FLAG_NONE;

                    if let Some(offset_size) = buffer_barrier.offset_size {
                        if offset_size.byte_offset != 0
                            || offset_size.size != buffer_barrier.buffer.buffer_def().size
                        {
                            unimplemented!("WARNING: DX12 doesn't support offset_size in buffer barriers, it requires extended barriers.");
                        }
                    }

                    //TODO: Partial barriers

                    let (state_before, state_after) = match buffer_barrier.queue_transition {
                        RafxBarrierQueueTransition::ReleaseTo(_) => (
                            buffer_barrier.src_state.into(),
                            d3d12::D3D12_RESOURCE_STATE_COMMON,
                        ),
                        RafxBarrierQueueTransition::AcquireFrom(_) => (
                            d3d12::D3D12_RESOURCE_STATE_COMMON,
                            buffer_barrier.dst_state.into(),
                        ),
                        RafxBarrierQueueTransition::None => (
                            buffer_barrier.src_state.into(),
                            buffer_barrier.dst_state.into(),
                        ),
                    };

                    barrier.Anonymous.Transition =
                        ManuallyDrop::new(d3d12::D3D12_RESOURCE_TRANSITION_BARRIER {
                            pResource: windows::core::ManuallyDrop::new(
                                buffer_barrier.buffer.dx12_buffer().unwrap().dx12_resource(),
                            ),
                            Subresource: d3d12::D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                            StateBefore: state_before,
                            StateAfter: state_after,
                        });

                    // log::info!(
                    //     "RESOURCE BARRIER Resource {:?} ({:?}) states {:?}->{:?} {:?}",
                    //     buffer_barrier.buffer.dx12_buffer().unwrap().dx12_resource(),
                    //     d3d12::D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                    //     buffer_barrier.src_state,
                    //     buffer_barrier.dst_state,
                    //     buffer_barrier.queue_transition
                    // );
                    //log::info!("{:?}", backtrace::Backtrace::new());

                    barriers.push(barrier);
                }
            }
        }

        for texture_barrier in texture_barriers {
            if texture_barrier
                .src_state
                .intersects(RafxResourceState::UNORDERED_ACCESS)
                && texture_barrier
                    .dst_state
                    .intersects(RafxResourceState::UNORDERED_ACCESS)
            {
                let mut barrier = d3d12::D3D12_RESOURCE_BARRIER::default();
                barrier.Type = d3d12::D3D12_RESOURCE_BARRIER_TYPE_UAV;
                barrier.Flags = d3d12::D3D12_RESOURCE_BARRIER_FLAG_NONE;
                barrier.Anonymous.UAV = ManuallyDrop::new(d3d12::D3D12_RESOURCE_UAV_BARRIER {
                    pResource: windows::core::ManuallyDrop::new(
                        texture_barrier
                            .texture
                            .dx12_texture()
                            .unwrap()
                            .dx12_resource(),
                    ),
                });
                barriers.push(barrier);
            } else {
                //TODO: Partial barriers

                let (state_before, state_after) = match texture_barrier.queue_transition {
                    RafxBarrierQueueTransition::ReleaseTo(_) => (
                        texture_barrier.src_state.into(),
                        d3d12::D3D12_RESOURCE_STATE_COMMON,
                    ),
                    RafxBarrierQueueTransition::AcquireFrom(_) => (
                        d3d12::D3D12_RESOURCE_STATE_COMMON,
                        texture_barrier.dst_state.into(),
                    ),
                    RafxBarrierQueueTransition::None => (
                        texture_barrier.src_state.into(),
                        texture_barrier.dst_state.into(),
                    ),
                };

                let create_barrier = |subresource| {
                    let mut barrier = d3d12::D3D12_RESOURCE_BARRIER::default();
                    barrier.Type = d3d12::D3D12_RESOURCE_BARRIER_TYPE_TRANSITION;
                    barrier.Flags = d3d12::D3D12_RESOURCE_BARRIER_FLAG_NONE;
                    barrier.Anonymous.Transition =
                        ManuallyDrop::new(d3d12::D3D12_RESOURCE_TRANSITION_BARRIER {
                            pResource: windows::core::ManuallyDrop::new(
                                texture_barrier
                                    .texture
                                    .dx12_texture()
                                    .unwrap()
                                    .dx12_resource(),
                            ),
                            Subresource: subresource,
                            StateBefore: state_before,
                            StateAfter: state_after,
                        });
                    barrier
                };

                if let Some(mip_slice) = texture_barrier.mip_slice {
                    if let Some(array_slice) = texture_barrier.array_slice {
                        // Handle both mip/array specified
                        let subresource = super::internal::dx12_subresource_index(
                            mip_slice,
                            array_slice,
                            0,
                            texture_barrier.texture.texture_def().mip_count,
                            texture_barrier.texture.texture_def().array_length,
                        );

                        barriers.push(create_barrier(subresource));
                    } else {
                        // Handle a single mip in all array slices
                        for array_slice in
                            0..texture_barrier.texture.texture_def().array_length as u16
                        {
                            let subresource = super::internal::dx12_subresource_index(
                                mip_slice,
                                array_slice,
                                0,
                                texture_barrier.texture.texture_def().mip_count,
                                texture_barrier.texture.texture_def().array_length,
                            );

                            barriers.push(create_barrier(subresource));
                        }
                    }
                } else {
                    if let Some(array_slice) = texture_barrier.array_slice {
                        // Handle all mips in a single array slice
                        for mip_slice in 0..texture_barrier.texture.texture_def().mip_count as u16 {
                            let subresource = super::internal::dx12_subresource_index(
                                mip_slice as u8,
                                array_slice,
                                0,
                                texture_barrier.texture.texture_def().mip_count,
                                texture_barrier.texture.texture_def().array_length,
                            );

                            barriers.push(create_barrier(subresource));
                        }
                    } else {
                        // all mips/array slices
                        barriers.push(create_barrier(
                            d3d12::D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                        ));
                    }
                }

                // println!(
                //     "RESOURCE BARRIER Resource {:?} (mip_slice: {:?} array_slice: {:?}) states {:?}->{:?}",
                //     texture_barrier.texture.dx12_texture().unwrap().dx12_resource(),
                //     texture_barrier.mip_slice,
                //     texture_barrier.array_slice,
                //     state_before,
                //     state_after
                // );
                //println!("{:?}", backtrace::Backtrace::new());
            }
        }

        if !barriers.is_empty() {
            let inner = self.inner.borrow_mut();
            unsafe {
                inner.command_list.ResourceBarrier(&barriers);
            }
        }

        Ok(())
    }

    pub fn cmd_copy_buffer_to_buffer(
        &self,
        src_buffer: &RafxBufferDx12,
        dst_buffer: &RafxBufferDx12,
        params: &RafxCmdCopyBufferToBufferParams,
    ) -> RafxResult<()> {
        let inner = self.inner.borrow_mut();

        unsafe {
            inner.command_list.CopyBufferRegion(
                dst_buffer.dx12_resource(),
                params.dst_byte_offset,
                src_buffer.dx12_resource(),
                params.src_byte_offset,
                params.size,
            )
        }

        Ok(())
    }

    pub fn cmd_copy_buffer_to_texture(
        &self,
        src_buffer: &RafxBufferDx12,
        dst_texture: &RafxTextureDx12,
        params: &RafxCmdCopyBufferToTextureParams,
    ) -> RafxResult<()> {
        let inner = self.inner.borrow_mut();
        //params.

        let subresource = super::internal::dx12_subresource_index(
            params.mip_level,
            params.array_layer,
            0,
            dst_texture.texture_def().mip_count,
            dst_texture.texture_def().array_length,
        );

        let desc = unsafe { dst_texture.dx12_resource().GetDesc() };

        let mut num_rows: u32 = 0;
        let mut row_size_in_bytes: u64 = 0;
        let mut total_bytes: u64 = 0;

        let mut placed_footprint = d3d12::D3D12_PLACED_SUBRESOURCE_FOOTPRINT::default();
        unsafe {
            self.queue
                .device_context()
                .d3d12_device()
                .GetCopyableFootprints(
                    &desc as *const _,
                    subresource,
                    1,
                    params.buffer_offset,
                    Some(&mut placed_footprint),
                    Some(&mut num_rows as *mut _),
                    Some(&mut row_size_in_bytes as *mut _),
                    Some(&mut total_bytes as *mut _),
                );
        }

        placed_footprint.Offset = params.buffer_offset;

        let mut src = d3d12::D3D12_TEXTURE_COPY_LOCATION::default();
        src.Type = d3d12::D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT;
        src.pResource = ::windows::core::ManuallyDrop::new(src_buffer.dx12_resource());
        src.Anonymous.PlacedFootprint = placed_footprint;

        let mut dst = d3d12::D3D12_TEXTURE_COPY_LOCATION::default();
        dst.Type = d3d12::D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
        dst.pResource = ::windows::core::ManuallyDrop::new(dst_texture.dx12_resource());
        dst.Anonymous.SubresourceIndex = subresource;

        unsafe {
            inner
                .command_list
                .CopyTextureRegion(&dst, 0, 0, 0, &src, None);
        }

        Ok(())
    }

    pub fn cmd_copy_texture_to_texture(
        &self,
        src_texture: &RafxTextureDx12,
        dst_texture: &RafxTextureDx12,
        params: &RafxCmdCopyTextureToTextureParams,
    ) -> RafxResult<()> {
        let inner = self.inner.borrow_mut();

        let mut src = d3d12::D3D12_TEXTURE_COPY_LOCATION::default();
        src.Type = d3d12::D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
        src.pResource = ::windows::core::ManuallyDrop::new(src_texture.dx12_resource());

        let mut dst = d3d12::D3D12_TEXTURE_COPY_LOCATION::default();
        dst.Type = d3d12::D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX;
        dst.pResource = ::windows::core::ManuallyDrop::new(dst_texture.dx12_resource());

        if let Some(array_slices) = params.array_slices {
            let src_slice = array_slices[0];
            let dst_slice = array_slices[1];

            src.Anonymous.SubresourceIndex = super::internal::dx12_subresource_index(
                params.src_mip_level,
                src_slice,
                0,
                src_texture.texture_def().mip_count,
                src_texture.texture_def().array_length,
            );

            dst.Anonymous.SubresourceIndex = super::internal::dx12_subresource_index(
                params.dst_mip_level,
                dst_slice,
                0,
                dst_texture.texture_def().mip_count,
                dst_texture.texture_def().array_length,
            );

            unsafe {
                inner
                    .command_list
                    .CopyTextureRegion(&dst, 0, 0, 0, &src, None);
            }
        } else {
            let array_length = src_texture.texture_def().array_length;
            assert_eq!(dst_texture.texture_def().array_length, array_length);

            for slice_index in 0..array_length {
                src.Anonymous.SubresourceIndex = super::internal::dx12_subresource_index(
                    params.src_mip_level,
                    slice_index as u16,
                    0,
                    src_texture.texture_def().mip_count,
                    src_texture.texture_def().array_length,
                );

                dst.Anonymous.SubresourceIndex = super::internal::dx12_subresource_index(
                    params.dst_mip_level,
                    slice_index as u16,
                    0,
                    dst_texture.texture_def().mip_count,
                    dst_texture.texture_def().array_length,
                );

                unsafe {
                    inner
                        .command_list
                        .CopyTextureRegion(&dst, 0, 0, 0, &src, None);
                }
            }
        }

        Ok(())
    }

    pub fn cmd_push_group_debug_name(
        &self,
        _name: impl AsRef<str>,
    ) {
        //TODO: IMPLEMENT ME
        //unimplemented!()
        // let mut inner = self.inner.borrow_mut();
        // if inner.debug_names_enabled {
        //     inner.group_debug_name_stack.push(name.as_ref().to_owned());
        //
        //     if let Some(encoder) = &inner.render_encoder {
        //         encoder.push_debug_group(name.as_ref());
        //     } else if let Some(encoder) = &inner.compute_encoder {
        //         encoder.push_debug_group(name.as_ref());
        //     } else if let Some(encoder) = &inner.blit_encoder {
        //         encoder.push_debug_group(name.as_ref());
        //     }
        // }
    }

    pub fn cmd_pop_group_debug_name(&self) {
        //TODO: IMPLEMENT ME
        //unimplemented!()
        // let mut inner = self.inner.borrow_mut();
        // if inner.debug_names_enabled {
        //     inner.group_debug_name_stack.pop();
        //
        //     if let Some(encoder) = &inner.render_encoder {
        //         encoder.pop_debug_group();
        //     } else if let Some(encoder) = &inner.compute_encoder {
        //         encoder.pop_debug_group();
        //     } else if let Some(encoder) = &inner.blit_encoder {
        //         encoder.pop_debug_group();
        //     }
        // }
    }
}
