use crate::RafxQueueType;
use windows::Win32::Graphics::Direct3D as d3d;
use windows::Win32::Graphics::Direct3D12 as d3d12;
use windows::Win32::Graphics::Dxgi as dxgi;

pub mod conversions;
pub mod descriptor_heap;
pub mod mipmap_resources;

pub fn queue_type_to_command_list_type(
    queue_type: RafxQueueType
) -> d3d12::D3D12_COMMAND_LIST_TYPE {
    match queue_type {
        RafxQueueType::Graphics => d3d12::D3D12_COMMAND_LIST_TYPE_DIRECT,
        RafxQueueType::Compute => d3d12::D3D12_COMMAND_LIST_TYPE_COMPUTE,
        RafxQueueType::Transfer => d3d12::D3D12_COMMAND_LIST_TYPE_COPY,
    }
}

pub fn dx12_subresource_index(
    mip_slice: u8,
    array_slice: u16,
    plane_slice: u32,
    mip_count: u32,
    array_length: u32,
) -> u32 {
    mip_slice as u32 + (array_slice as u32 * mip_count) + (plane_slice * mip_count * array_length)
}
