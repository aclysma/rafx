type FrameIndex = u32;
type AssetUuid = u32;

trait AssetUploader {

    /// When called, assets that were unloaded during the given frame are destroyed
    fn on_gpu_frame_present_complete(&mut self, frame_index: FrameIndex);

    //
    // Calls from the asset system
    //

    /// Initiates uploading data for the given asset. When complete, load_op.complete() will be
    /// called
    fn update_asset(&mut self, uuid: AssetUuid, data: Vec<u8>, version: u32);

    /// Sets the given version as most recently committed, which means future calls to
    /// on_gpu_begin_read_for_frame will pin the frame to the committed data
    fn commit_asset_version(&mut self, version: u32);

    /// Queues resources to be released once the current frame ends (most recent index passed to
    /// on_gpu_begin_read_for_frame). Future frames will not have access to this resource at all.
    fn free(&mut self, uuid: AssetUuid, last_used_by_frame_index: FrameIndex);


    // //
    // // Fetches data
    // //
    //
    // /// Returns the GPU resource associated with the given frame.
    // ///
    // /// WARNING: Do not mix resources from before and after an asset loading tick.
    // fn get_resource(&self, uuid: AssetUuid) -> Vec<u8>;
}

use ash::vk;
use renderer_shell_vulkan::VkImage;
use renderer_shell_vulkan::VkBuffer;
use crate::image_utils::DecodedTexture;
use std::mem::ManuallyDrop;
use ash::prelude::*;
use fnv::FnvHashMap;
use ash::version::DeviceV1_0;

type TextureHandle = u32;

// Assets that need to be copied to a staging buffer
struct PendingUpload {
    // UUID
    uuid: AssetUuid,
    data: Vec<u8>
}

// Assets that are in a staging buffer and have a command issued to copy into a command buffer
struct InProgressUpload {
    // UUIDs included in the command buffer
    // Fence that can be checked
    // staging buffer
    // device buffer
    uuid: AssetUuid,
    data: Vec<u8>
}

// Assets that have been uploaded
struct CompletedUpload {
    // device buffer
    uuid: AssetUuid,
    data: TextureHandle
}

#[derive(Default)]
struct TextureAssetUploader {
    pending_uploads: Vec<PendingUpload>,
    in_progress_uploads: Vec<InProgressUpload>,
    completed_uploads: Vec<CompletedUpload>,
    pending_removes: FnvHashMap<FrameIndex, Vec<AssetUuid>>,

    assets: FnvHashMap<AssetUuid, TextureHandle>,

    //current_frame: u32,
}

impl TextureAssetUploader {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn update(&mut self) {
        for pending_upload in self.pending_uploads.drain(..) {
            self.in_progress_uploads.push(InProgressUpload {
                uuid: pending_upload.uuid,
                data: pending_upload.data
            });
        }

        for in_progress_upload in self.in_progress_uploads.drain(..) {
            self.completed_uploads.push(CompletedUpload {
                uuid: in_progress_upload.uuid,
                data: 0
            });
        }
    }

    /// Returns the GPU resource associated with the given frame.
    ///
    /// WARNING: Do not mix resources from before and after an asset loading tick.
    fn get_resource(&self, uuid: AssetUuid) -> Option<TextureHandle> {
        // Try to fetch it from assets
        self.assets.get(&uuid).map(|x| *x)
    }
}

impl AssetUploader for TextureAssetUploader {
    fn on_gpu_frame_present_complete(&mut self, frame_index: FrameIndex) {
        // Drop everything in pending_unloads under frame_index out of the asset hashmap
        let assets_to_remove = self.pending_removes.get(&frame_index);
        if let Some(assets_to_remove) = assets_to_remove {
            for asset in assets_to_remove {
                self.assets.remove(asset);
            }
        }

        self.pending_removes.remove(&frame_index);
    }

    fn update_asset(&mut self, uuid: AssetUuid, data: Vec<u8>, version: u32) {
        // Push the data into pending_uploads.. either kick off a task to do the upload or
        // wait until later to kick it off as a batch

        self.pending_uploads.push(PendingUpload {
            uuid,
            data
        });
    }

    /// Sets the given version as most recently committed, which means future calls to
    /// on_gpu_begin_read_for_frame will pin the frame to the committed data
    fn commit_asset_version(&mut self, version: u32) {
        // Copy completed uploads into the assets hash map
        for completed_upload in self.completed_uploads.drain(..) {
            self.assets.insert(completed_upload.uuid, completed_upload.data);
        }
    }

    /// Queues resources to be released once the current frame ends (most recent index passed to
    /// on_gpu_begin_read_for_frame). Future frames will not have access to this resource at all.
    fn free(&mut self, uuid: AssetUuid, last_used_by_frame_index: FrameIndex) {
        // Push the asset into pending_unloads
        self.pending_removes.entry(last_used_by_frame_index).or_default().push(uuid);
    }
}
