use std::mem;
use ash::{vk, Device};
use ash::prelude::VkResult;
use std::ffi::CString;
use std::mem::ManuallyDrop;

use ash::version::DeviceV1_0;

use renderer_shell_vulkan::{
    VkDevice, VkUpload, VkTransferUpload, VkTransferUploadState, VkResourceDropSink,
    VkDescriptorPoolAllocator, VkDeviceContext,
};
use renderer_shell_vulkan::VkSwapchain;
use renderer_shell_vulkan::offset_of;
use renderer_shell_vulkan::SwapchainInfo;
use renderer_shell_vulkan::VkQueueFamilyIndices;
use renderer_shell_vulkan::VkBuffer;
use renderer_shell_vulkan::util;

use renderer_shell_vulkan::VkImage;
use image::error::ImageError::Decoding;
use std::process::exit;
use image::{GenericImageView, ImageFormat, load};
use ash::vk::{ShaderStageFlags, DescriptorPoolResetFlags};
use crate::image_utils::DecodedTexture;
use std::collections::VecDeque;
use imgui::FontSource::DefaultFontData;
use crossbeam_channel::{Receiver, Sender};
use std::time::Duration;
use imgui::Image;
use std::error::Error;
use std::num::Wrapping;
use itertools::max;
use renderer_shell_vulkan::cleanup::CombinedDropSink;
use crate::asset_storage::ResourceHandle;
use atelier_assets::core::AssetUuid;
use crate::resource_managers::{SpriteResourceManager, MaterialResourceManager};
use crate::pipeline::gltf::{MeshAsset, MaterialAsset};
use crate::pipeline::image::ImageAsset;

/// Represents an image that will replace another image
pub struct LoadingMeshPartRenderInfo {
    pub index_offset: u32,
    pub index_size: u32,
    pub vertex_offset: u32,
    pub vertex_size: u32,
    pub material: Option<AssetUuid>,
}

pub struct LoadingMeshRenderInfo {
    pub mesh_parts: Vec<LoadingMeshPartRenderInfo>,
    pub buffer: ManuallyDrop<VkBuffer>,
}

pub struct MeshUpdate {
    pub meshes: Vec<LoadingMeshRenderInfo>,
    pub resource_handles: Vec<ResourceHandle<MeshAsset>>,
}

pub struct MeshPartRenderInfo {
    pub index_offset: u32,
    pub index_size: u32,
    pub vertex_offset: u32,
    pub vertex_size: u32,
    pub material_handle: Option<ResourceHandle<MaterialAsset>>,
}

pub struct MeshRenderInfo {
    pub mesh_parts: Vec<MeshPartRenderInfo>,
    pub buffer: ManuallyDrop<VkBuffer>,
}

/// Represents the current state of the mesh and the GPU resources associated with it
pub struct Mesh {
    pub render_info: MeshRenderInfo,
}

/// Keeps track of meshes and manages descriptor sets that allow shaders to bind to images
/// and use them
pub struct VkMeshResourceManager {
    device_context: VkDeviceContext,

    // The raw texture resources
    meshes: Vec<Option<Mesh>>,
    drop_sink: CombinedDropSink,

    // The descriptor set layout, pools, and sets
    // descriptor_set_layout: vk::DescriptorSetLayout,
    // descriptor_pool_allocator: VkDescriptorPoolAllocator,
    // descriptor_pool: vk::DescriptorPool,
    // descriptor_sets: Vec<vk::DescriptorSet>,

    // For sending image updates in a thread-safe manner
    mesh_update_tx: Sender<MeshUpdate>,
    mesh_update_rx: Receiver<MeshUpdate>,
}

impl VkMeshResourceManager {
    // pub fn descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
    //     self.descriptor_set_layout
    // }
    //
    // pub fn descriptor_sets(&self) -> &Vec<vk::DescriptorSet> {
    //     &self.descriptor_sets
    // }

    pub fn mesh_update_tx(&self) -> &Sender<MeshUpdate> {
        &self.mesh_update_tx
    }

    pub fn meshes(&self) -> &Vec<Option<Mesh>> {
        &self.meshes
    }

    pub fn new(
        device_context: &VkDeviceContext,
        max_frames_in_flight: u32,
    ) -> VkResult<Self> {
        let meshes = Vec::new();

        //
        // Descriptors
        //
        // let descriptor_set_layout = Self::create_descriptor_set_layout(device_context.device())?;
        // let mut descriptor_pool_allocator = VkDescriptorPoolAllocator::new(
        //     max_frames_in_flight,
        //     max_frames_in_flight + 1,
        //     |device| Self::create_descriptor_pool(device),
        // );
        // let descriptor_pool = descriptor_pool_allocator.allocate_pool(device_context.device())?;
        // let descriptor_sets = Self::create_descriptor_set(
        //     device_context.device(),
        //     descriptor_set_layout,
        //     &descriptor_pool,
        //     &meshes,
        // )?;

        let (mesh_update_tx, mesh_update_rx) = crossbeam_channel::unbounded();

        let drop_sink = CombinedDropSink::new(max_frames_in_flight + 1);

        Ok(VkMeshResourceManager {
            device_context: device_context.clone(),
            // descriptor_set_layout,
            // descriptor_pool_allocator,
            // descriptor_pool,
            // descriptor_sets,
            meshes,
            drop_sink,
            mesh_update_tx,
            mesh_update_rx,
        })
    }

    pub fn update(
        &mut self,
        material_resource_manager: &MaterialResourceManager,
    ) {
        // This will handle any resources that need to be dropped
        // self.descriptor_pool_allocator
        //     .update(self.device_context.device());
        self.drop_sink
            .on_frame_complete(self.device_context.device());

        // Check if we have any image updates to process
        //TODO: This may need to be deferred until a commit, and the commit may be to update to a
        // particular version of the assets
        self.try_update_meshes(material_resource_manager);
    }

    /// Checks if there are pending image updates, and if there are, regenerates the descriptor sets
    fn try_update_meshes(
        &mut self,
        material_resource_manager: &MaterialResourceManager,
    ) {
        //let mut has_update = false;
        while let Ok(update) = self.mesh_update_rx.recv_timeout(Duration::from_secs(0)) {
            self.do_update_meshes(update, material_resource_manager);
            //has_update = true;
        }

        // if has_update {
        //     self.refresh_descriptor_sets();
        // }
    }

    /// Runs through the incoming image updates and applies them to the list of meshes
    fn do_update_meshes(
        &mut self,
        mesh_update: MeshUpdate,
        material_resource_manager: &MaterialResourceManager,
    ) {
        let mut max_index = self.meshes.len();
        for resource_handle in &mesh_update.resource_handles {
            max_index = max_index.max(resource_handle.index() as usize + 1);
        }

        self.meshes.resize_with(max_index, || None);

        let mut old_meshes = vec![];
        for (i, render_info) in mesh_update.meshes.into_iter().enumerate() {
            let resource_handle = mesh_update.resource_handles[i];

            println!(
                "UPLOAD MESH {}",
                render_info.buffer.allocation_info.get_size()
            );

            // let image_view =
            //     Self::create_texture_image_view(self.device_context.device(), &image.image);

            let mut mesh_parts: Vec<_> = render_info
                .mesh_parts
                .iter()
                .map(|loading_mesh_part| {
                    let material = loading_mesh_part.material;
                    let material_handle = material.and_then(|material| {
                        material_resource_manager.material_handle_by_uuid(&material)
                    });

                    println!(
                        "do_update_meshes {:?} {:?}",
                        loading_mesh_part.material, material_handle
                    );

                    // let m = material
                    //     .and_then(|material| sprite_resource_manager.sprite_handle_by_uuid(&material))
                    //     .and_then(|handle| sprite_resource_manager.descriptor_sets()[handle.index()]);

                    MeshPartRenderInfo {
                        vertex_size: loading_mesh_part.vertex_size,
                        vertex_offset: loading_mesh_part.vertex_offset,
                        index_size: loading_mesh_part.index_size,
                        index_offset: loading_mesh_part.index_offset,
                        material_handle,
                    }
                })
                .collect();

            let render_info = MeshRenderInfo {
                mesh_parts: mesh_parts,
                buffer: render_info.buffer,
            };

            // Do a swap so if there is an old mesh we can properly destroy it
            let mut mesh = Some(Mesh { render_info });
            std::mem::swap(
                &mut mesh,
                &mut self.meshes[resource_handle.index() as usize],
            );
            if mesh.is_some() {
                old_meshes.push(mesh);
            }
        }

        // retire old meshes
        for mesh in old_meshes.drain(..) {
            let mesh = mesh.unwrap();
            self.drop_sink.retire_buffer(mesh.render_info.buffer);
        }
    }

    // fn refresh_descriptor_sets(&mut self) -> VkResult<()> {
    //     self.descriptor_pool_allocator
    //         .retire_pool(self.descriptor_pool);
    //
    //     let descriptor_pool = self
    //         .descriptor_pool_allocator
    //         .allocate_pool(self.device_context.device())?;
    //     let descriptor_sets = Self::create_descriptor_set(
    //         self.device_context.device(),
    //         self.descriptor_set_layout,
    //         &descriptor_pool,
    //         &self.meshes,
    //     )?;
    //
    //     self.descriptor_pool = descriptor_pool;
    //     self.descriptor_sets = descriptor_sets;
    //
    //     Ok(())
    // }

    // pub fn create_texture_image_view(
    //     logical_device: &ash::Device,
    //     image: &vk::Image,
    // ) -> vk::ImageView {
    //     let subresource_range = vk::ImageSubresourceRange::builder()
    //         .aspect_mask(vk::ImageAspectFlags::COLOR)
    //         .base_mip_level(0)
    //         .level_count(1)
    //         .base_array_layer(0)
    //         .layer_count(1);
    //
    //     let image_view_info = vk::ImageViewCreateInfo::builder()
    //         .image(*image)
    //         .view_type(vk::ImageViewType::TYPE_2D)
    //         .format(vk::Format::R8G8B8A8_UNORM)
    //         .subresource_range(*subresource_range);
    //
    //     unsafe {
    //         logical_device
    //             .create_image_view(&image_view_info, None)
    //             .unwrap()
    //     }
    // }

    // fn create_descriptor_set_layout(
    //     logical_device: &ash::Device
    // ) -> VkResult<vk::DescriptorSetLayout> {
    //     let descriptor_set_layout_bindings = [vk::DescriptorSetLayoutBinding::builder()
    //         .binding(0)
    //         .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
    //         .descriptor_count(1)
    //         .stage_flags(vk::ShaderStageFlags::FRAGMENT)
    //         .build()];
    //
    //     let descriptor_set_layout_create_info =
    //         vk::DescriptorSetLayoutCreateInfo::builder().bindings(&descriptor_set_layout_bindings);
    //
    //     unsafe {
    //         logical_device.create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
    //     }
    // }
    //
    // fn create_descriptor_pool(logical_device: &ash::Device) -> VkResult<vk::DescriptorPool> {
    //     const MAX_MESHES: u32 = 1000;
    //
    //     let pool_sizes = [vk::DescriptorPoolSize::builder()
    //         .ty(vk::DescriptorType::SAMPLED_IMAGE)
    //         .descriptor_count(MAX_MESHES)
    //         .build()];
    //
    //     let descriptor_pool_info = vk::DescriptorPoolCreateInfo::builder()
    //         .pool_sizes(&pool_sizes)
    //         .max_sets(MAX_MESHES);
    //
    //     unsafe { logical_device.create_descriptor_pool(&descriptor_pool_info, None) }
    // }
    //
    // fn create_descriptor_set(
    //     logical_device: &ash::Device,
    //     descriptor_set_layout: vk::DescriptorSetLayout,
    //     descriptor_pool: &vk::DescriptorPool,
    //     meshes: &[Option<Mesh>],
    // ) -> VkResult<Vec<vk::DescriptorSet>> {
    //     let descriptor_set_layouts = vec![descriptor_set_layout; meshes.len()];
    //
    //     let descriptor_sets = if !meshes.is_empty() {
    //         let alloc_info = vk::DescriptorSetAllocateInfo::builder()
    //             .descriptor_pool(*descriptor_pool)
    //             .set_layouts(descriptor_set_layouts.as_slice());
    //
    //         unsafe { logical_device.allocate_descriptor_sets(&alloc_info) }?
    //     } else {
    //         vec![]
    //     };
    //
    //     for (image_index, mesh) in meshes.iter().enumerate() {
    //         if let Some(mesh) = mesh.as_ref() {
    //             // let mut descriptor_writes = Vec::with_capacity(meshes.len());
    //             // let image_view_descriptor_image_info = vk::DescriptorImageInfo::builder()
    //             //     .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
    //             //     .image_view(mesh.image_view)
    //             //     .build();
    //             //
    //             // descriptor_writes.push(
    //             //     vk::WriteDescriptorSet::builder()
    //             //         .dst_set(descriptor_sets[image_index])
    //             //         .dst_binding(0)
    //             //         .dst_array_element(0)
    //             //         .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
    //             //         .image_info(&[image_view_descriptor_image_info])
    //             //         .build(),
    //             // );
    //             unsafe {
    //                 //logical_device.update_descriptor_sets(&descriptor_writes, &[]);
    //             }
    //         }
    //     }
    //
    //     Ok(descriptor_sets)
    // }
}

impl Drop for VkMeshResourceManager {
    fn drop(&mut self) {
        log::debug!("destroying VkMeshResourceManager");

        unsafe {
            // self.descriptor_pool_allocator
            //     .retire_pool(self.descriptor_pool);
            // self.descriptor_pool_allocator
            //     .destroy(self.device_context.device());

            // self.device_context
            //     .device()
            //     .destroy_descriptor_set_layout(self.descriptor_set_layout, None);

            for mesh in self.meshes.drain(..) {
                if let Some(mesh) = mesh {
                    self.drop_sink.retire_buffer(mesh.render_info.buffer);
                    // self.drop_sink.retire_image(mesh.mesh);
                    // self.drop_sink.retire_image_view(mesh.image_view);
                }
            }
            self.drop_sink.destroy(self.device_context.device());
        }

        log::debug!("destroyed VkMeshResourceManager");
    }
}
