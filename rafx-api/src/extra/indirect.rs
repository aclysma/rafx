//! Contents of this file are to help emulate gl_InstanceIndex on DX12. Metal follows the same
//! convention as vulkan.

use crate::{
    RafxBuffer, RafxCommandBuffer, RafxDeviceContext, RafxDrawIndexedIndirectCommand,
    RafxDrawIndirectCommand, RafxResult, RafxRootSignature, RafxShaderStageFlags,
};

#[cfg(feature = "rafx-dx12")]
use windows::Win32::Graphics::Direct3D12 as d3d12;

// In order to use indirect commands we need to create a command signature that is compatible
// with the root signature that will be used
#[cfg(feature = "rafx-dx12")]
fn create_indirect_draw_with_push_constant_command_signature(
    device: &d3d12::ID3D12Device,
    root_signature: &d3d12::ID3D12RootSignature,
    indexed: bool,
) -> RafxResult<d3d12::ID3D12CommandSignature> {
    let mut sig = d3d12::D3D12_COMMAND_SIGNATURE_DESC::default();
    let mut draw_arg = d3d12::D3D12_INDIRECT_ARGUMENT_DESC::default();
    let mut root_constant_arg = d3d12::D3D12_INDIRECT_ARGUMENT_DESC::default();

    if !indexed {
        draw_arg.Type = d3d12::D3D12_INDIRECT_ARGUMENT_TYPE_DRAW;
        sig.ByteStride = std::mem::size_of::<RafxDrawIndirectCommand>() as u32 + 4;
    } else {
        draw_arg.Type = d3d12::D3D12_INDIRECT_ARGUMENT_TYPE_DRAW_INDEXED;
        sig.ByteStride = std::mem::size_of::<RafxDrawIndexedIndirectCommand>() as u32 + 4;
    }

    root_constant_arg.Type = d3d12::D3D12_INDIRECT_ARGUMENT_TYPE_CONSTANT;
    root_constant_arg.Anonymous.Constant.RootParameterIndex = 0;
    root_constant_arg.Anonymous.Constant.DestOffsetIn32BitValues = 0;
    root_constant_arg.Anonymous.Constant.Num32BitValuesToSet = 1;

    sig.NumArgumentDescs = 2;
    let args = [root_constant_arg, draw_arg];
    sig.pArgumentDescs = args.as_ptr();

    let mut result: Option<d3d12::ID3D12CommandSignature> = None;

    unsafe {
        device.CreateCommandSignature(&sig, root_signature, &mut result)?;
    }

    Ok(result.unwrap())
}

// Corresponds 1:1 with VkDrawIndirectCommand, MTLDrawPrimitivesIndirectArguments,
// D3D12_DRAW_ARGUMENTS, but adds a push constant for DX12
pub struct RafxDrawIndirectCommandWithPushConstant {
    pub push_constant: u32,
    pub command: RafxDrawIndirectCommand,
}

// Corresponds 1:1 with VkDrawIndexedIndirectCommand, MTLDrawIndexedPrimitivesIndirectArguments,
// D3D12_DRAW_INDEXED_ARGUMENTS, but adds a push constant for DX12
pub struct RafxDrawIndexedIndirectCommandWithPushConstant {
    pub push_constant: u32,
    pub command: RafxDrawIndexedIndirectCommand,
}

// Size of an indirect draw command compatible with the given device context
pub fn indirect_command_size(_device_context: &RafxDeviceContext) -> u64 {
    #[cfg(feature = "rafx-dx12")]
    if _device_context.is_dx12() {
        return std::mem::size_of::<RafxDrawIndirectCommand>() as u64 + 4;
    }

    std::mem::size_of::<RafxDrawIndirectCommand>() as u64
}

// Size of an indexed indirect draw command compatible with the given device context
pub fn indexed_indirect_command_size(_device_context: &RafxDeviceContext) -> u64 {
    #[cfg(feature = "rafx-dx12")]
    if _device_context.is_dx12() {
        return std::mem::size_of::<RafxDrawIndexedIndirectCommand>() as u64 + 4;
    }

    std::mem::size_of::<RafxDrawIndexedIndirectCommand>() as u64
}

//TODO: Support a non-indexed version of RafxIndexedIndirectCommandSignature and
// RafxIndexedIndirectCommandEncoder

/// A helper object for doing indirect draw on DX12/Metal/Vulkan in a compatible way. We supply a
/// push constant on DX12 only to emulate gl_InstanceIndex on DX12. This helper object is mostly
/// a no-op for vulkan/metal.
#[derive(Clone)]
pub struct RafxIndexedIndirectCommandSignature {
    _root_signature: RafxRootSignature,
    #[cfg(feature = "rafx-dx12")]
    dx12_indirect_command_signature: Option<d3d12::ID3D12CommandSignature>,
}

impl RafxIndexedIndirectCommandSignature {
    pub fn new(
        root_signature: &RafxRootSignature,
        _shader_flags: RafxShaderStageFlags,
    ) -> RafxResult<Self> {
        #[cfg(feature = "rafx-dx12")]
        if let Some(root_signature_dx12) = root_signature.dx12_root_signature() {
            let descriptor = root_signature_dx12.find_push_constant_descriptor(_shader_flags).ok_or_else(|| crate::RafxError::StringError(format!(
                "Tried to create a RafxIndexedIndirectCommandSignature for shader flags {:?} but no push constants were found",
                _shader_flags
            )))?;

            let command_signature = create_indirect_draw_with_push_constant_command_signature(
                root_signature_dx12.device_context().d3d12_device(),
                root_signature_dx12.dx12_root_signature(),
                true,
            )?;

            return Ok(RafxIndexedIndirectCommandSignature {
                _root_signature: root_signature.clone(),
                dx12_indirect_command_signature: Some(command_signature),
            });
        }

        Ok(RafxIndexedIndirectCommandSignature {
            _root_signature: root_signature.clone(),
            #[cfg(feature = "rafx-dx12")]
            dx12_indirect_command_signature: None,
        })
    }

    // equivalent to cmd_draw_indexed_indirect
    pub fn draw_indexed_indirect(
        &self,
        command_buffer: &RafxCommandBuffer,
        indirect_buffer: &RafxBuffer,
        indirect_buffer_offset_in_bytes: u32,
        draw_count: u32,
    ) -> RafxResult<()> {
        // Special DX12 path
        #[cfg(feature = "rafx-dx12")]
        if let Some(dx12_command_buffer) = command_buffer.dx12_command_buffer() {
            let command_list = dx12_command_buffer.dx12_graphics_command_list();
            unsafe {
                let command_signature = self.dx12_indirect_command_signature.as_ref().unwrap();
                assert!(
                    indirect_buffer.buffer_def().size as u32 - indirect_buffer_offset_in_bytes
                        >= 24 * draw_count
                );

                command_list.ExecuteIndirect(
                    command_signature,
                    draw_count,
                    indirect_buffer.dx12_buffer().unwrap().dx12_resource(),
                    indirect_buffer_offset_in_bytes as u64,
                    None,
                    0,
                );
            }

            return Ok(());
        }

        // Path for non-DX12
        command_buffer.cmd_draw_indexed_indirect(
            indirect_buffer,
            indirect_buffer_offset_in_bytes,
            draw_count,
        )
    }
}

/// Helper object for writing indirect draws into a buffer. Abstracts over DX12 requiring an
/// extra 4 bytes to set a push constant
pub struct RafxIndexedIndirectCommandEncoder<'a> {
    // We keep a ref to the buffer because we write into mapped memory behind a cached pointer
    _buffer: &'a RafxBuffer,
    #[cfg(feature = "rafx-dx12")]
    is_dx12: bool,
    mapped_memory: *mut u8,
    command_count: usize,
}

impl<'a> RafxIndexedIndirectCommandEncoder<'a> {
    pub fn new(buffer: &'a RafxBuffer) -> Self {
        #[cfg(not(feature = "rafx-dx12"))]
        let is_dx12 = false;

        #[cfg(feature = "rafx-dx12")]
        let is_dx12 = buffer.dx12_buffer().is_some();

        let command_size = if is_dx12 {
            std::mem::size_of::<RafxDrawIndexedIndirectCommand>() + 4
        } else {
            std::mem::size_of::<RafxDrawIndexedIndirectCommand>()
        };

        let command_count = (buffer.buffer_def().size as usize / command_size) as usize;
        RafxIndexedIndirectCommandEncoder {
            _buffer: buffer,
            #[cfg(feature = "rafx-dx12")]
            is_dx12,
            mapped_memory: buffer.mapped_memory().unwrap(),
            command_count,
        }
    }

    pub fn set_command(
        &self,
        index: usize,
        command: RafxDrawIndexedIndirectCommand,
    ) {
        assert!(index < self.command_count);
        unsafe {
            #[cfg(feature = "rafx-dx12")]
            if self.is_dx12 {
                let ptr = self.mapped_memory as *mut RafxDrawIndexedIndirectCommandWithPushConstant;
                let push_constant = command.first_instance;
                *ptr.add(index) = RafxDrawIndexedIndirectCommandWithPushConstant {
                    command,
                    push_constant,
                };

                return;
            }

            // If we don't have the special dx12 case, use the default command type
            let ptr = self.mapped_memory as *mut RafxDrawIndexedIndirectCommand;
            *ptr.add(index) = command;
        }
    }
}
