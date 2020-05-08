use renderer_shell_vulkan::{VkTransferUploadState, VkDevice, VkDeviceContext, VkTransferUpload, VkImage, VkBuffer};
use crossbeam_channel::{Sender, Receiver};
use ash::prelude::VkResult;
use std::time::Duration;
use crate::image_utils::{enqueue_load_images, DecodedTexture};
use crate::renderpass::sprite::ImageUpdate;
use std::mem::{ManuallyDrop, align_of};
use crate::asset_storage::{ResourceHandle, StorageUploader};
use crate::image_importer::ImageAsset;
use std::error::Error;
use atelier_assets::loader::{LoadHandle, AssetLoadOp};
use fnv::FnvHashMap;
use std::sync::Arc;
use image::load;

use crate::upload::{PendingImageUpload, PendingBufferUpload};
use crate::upload::BufferUploadOpResult;
use crate::upload::BufferUploadOpAwaiter;
use crate::gltf_importer::{MeshAsset, Vertex};
use crate::renderpass::mesh::mesh_resource_manager::{MeshUpdate, MeshPartRenderInfo, MeshRenderInfo};

pub struct PushBufferResult {
    offset: usize,
    size: usize
}

pub struct PushBufferSizeCalculator {
    required_size: usize
}

impl PushBufferSizeCalculator {
    pub fn new() -> Self {
        PushBufferSizeCalculator {
            required_size: 0
        }
    }

    pub fn push_bytes(
        &mut self,
        data: &[u8],
        required_alignment: usize,
    ) {
        self.push(data, required_alignment)
    }

    pub fn push<T>(&mut self, data: &[T], required_alignment: usize) {
        self.required_size = ((self.required_size + required_alignment - 1) / required_alignment) * required_alignment;
        self.required_size += (data.len() * std::mem::size_of::<T>());
    }

    pub fn required_size(&self) -> usize {
        self.required_size
    }
}

pub struct PushBuffer {
    data: Vec<u8>
}

impl PushBuffer {
    pub fn new(size_hint: usize) -> Self {
        PushBuffer {
            data: Vec::with_capacity(size_hint),
        }
    }

    pub fn push_bytes(
        &mut self,
        data: &[u8],
        required_alignment: usize,
    ) -> PushBufferResult {
        // Figure out where in the buffer to write
        let span_begin = ((self.data.len() + required_alignment - 1) / required_alignment) * required_alignment;
        let span_end = span_begin + data.len();

        // Resize the buffer and copy the data
        self.data.resize(span_end, 0);
        self.data[span_begin..span_end].copy_from_slice(data);

        // Return the offset
        PushBufferResult {
            offset: span_begin,
            size: data.len()
        }
    }

    pub fn push<T>(&mut self, data: &[T], required_alignment: usize) -> PushBufferResult {
        let ptr: *const u8 = data.as_ptr() as *const u8;
        let slice: &[u8] = unsafe {
            std::slice::from_raw_parts(ptr, std::mem::size_of::<T>() * data.len())
        };

        self.push_bytes(slice, required_alignment)
    }
}


struct PendingMeshUpdate {
    awaiter: BufferUploadOpAwaiter,
    mesh_parts: Vec<MeshPartRenderInfo>
}


// This is registered with the asset storage which lets us hook when assets are updated
pub struct MeshUploader {
    upload_tx: Sender<PendingBufferUpload>,
    mesh_update_tx: Sender<MeshUpdate>,
    pending_updates: FnvHashMap<LoadHandle, FnvHashMap<u32, PendingMeshUpdate>>
}

impl MeshUploader {
    pub fn new(
        upload_tx: Sender<PendingBufferUpload>,
        mesh_update_tx: Sender<MeshUpdate>
    ) -> Self {
        MeshUploader {
            upload_tx,
            mesh_update_tx,
            pending_updates: Default::default()
        }
    }
}

// This sends the texture to the uploader. The uploader will batch uploads together when update()
// is called on it. When complete, the uploader will send the image handle back via a channel
impl StorageUploader<MeshAsset> for MeshUploader {
    fn update_asset(
        &mut self,
        load_handle: LoadHandle,
        load_op: AssetLoadOp,
        _resource_handle: ResourceHandle<MeshAsset>,
        version: u32,
        asset: &MeshAsset,
    ) {
        let (upload_op, awaiter) = crate::upload::create_upload_op();

        //
        // Determine size of buffer needed
        //
        const REQUIRED_ALIGNMENT : usize = 16;
        let mut storage_calculator = PushBufferSizeCalculator::new();
        for mesh_part in &asset.mesh_parts {
            storage_calculator.push(&mesh_part.indices, REQUIRED_ALIGNMENT);
            storage_calculator.push(&mesh_part.vertices, REQUIRED_ALIGNMENT);
        }

        //
        // Concatenate vertex/index data for all mesh parts into a buffer
        //
        let mut mesh_part_render_infos = Vec::with_capacity(asset.mesh_parts.len());
        let mut combined_mesh_data = PushBuffer::new(storage_calculator.required_size());
        for mesh_part in &asset.mesh_parts {
            let index = combined_mesh_data.push(&mesh_part.indices, REQUIRED_ALIGNMENT);
            let vertex = combined_mesh_data.push(&mesh_part.vertices, REQUIRED_ALIGNMENT);

            mesh_part_render_infos.push(MeshPartRenderInfo {
                index_offset: index.offset as u32,
                index_size: index.size as u32,
                vertex_offset: vertex.offset as u32,
                vertex_size: vertex.size as u32,
                material: 0
            });
        }

        let pending_update = PendingMeshUpdate {
            awaiter,
            mesh_parts: mesh_part_render_infos
        };

        self.pending_updates.entry(load_handle).or_default().insert(version,pending_update);

        self.upload_tx
            .send(PendingBufferUpload {
                load_op,
                upload_op,
                data: combined_mesh_data.data,
            })
            .unwrap(); //TODO: Better error handling
    }

    fn commit_asset_version(
        &mut self,
        load_handle: LoadHandle,
        resource_handle: ResourceHandle<MeshAsset>,
        version: u32
    ) {
        if let Some(versions) = self.pending_updates.get_mut(&load_handle) {
            if let Some(pending_update) = versions.remove(&version) {
                let awaiter = pending_update.awaiter;

                // We assume that if commit_asset_version is being called the awaiter is signaled
                // and has a valid result
                let value = awaiter.receiver().recv_timeout(Duration::from_secs(0)).unwrap();
                match value {
                    BufferUploadOpResult::UploadComplete(buffer) => {
                        log::info!("Commit asset {:?} {:?}", load_handle, version);

                        let mesh_render_info = MeshRenderInfo {
                            buffer,
                            mesh_parts: pending_update.mesh_parts
                        };

                        self.mesh_update_tx.send(MeshUpdate {
                            meshes: vec![mesh_render_info],
                            resource_handles: vec![resource_handle]
                        });
                    },
                    BufferUploadOpResult::UploadError => unreachable!(),
                    BufferUploadOpResult::UploadDrop => unreachable!(),
                }
            } else {
                log::error!("Could not find awaiter for asset version {:?} {}", load_handle, version);
            }
        } else {
            log::error!("Could not find awaiter for {:?} {}", load_handle, version);
        }
    }

    fn free(
        &mut self,
        load_handle: LoadHandle,
        resource_handle: ResourceHandle<MeshAsset>,
    ) {
        //TODO: We are not unloading images
        self.pending_updates.remove(&load_handle);
    }
}

