

//
// trait AssetUploader {
//
//     /// When called, assets that were unloaded during the given frame are destroyed
//     fn on_gpu_end_read_for_frame(frame_index: u32);
//
//     //
//     // Calls from the asset system
//     //
//
//     /// Initiates uploading data for the given asset. When complete, load_op.complete() will be
//     /// called
//     fn update_asset(uuid: u32, data: Vec<u8>, version: u32);
//
//     /// Sets the given version as most recently committed, which means future calls to
//     /// on_gpu_begin_read_for_frame will pin the frame to the committed data
//     fn commit_asset_version(version: u32);
//
//     /// Queues resources to be released once the current frame ends (most recent index passed to
//     /// on_gpu_begin_read_for_frame). Future frames will not have access to this resource at all.
//     fn free(uuid: u32, last_used_by_frame_index: u32);
//
//
//     //
//     // Fetches data
//     //
//
//     /// Returns the GPU resource associated with the given frame.
//     ///
//     /// WARNING: Do not mix resources from before and after an asset loading tick.
//     fn get_resource(uuid: u32) -> Vec<u8>;
// }


// trait AssetUploader {
//     //
//     // These are called when we start preparing a new frame and when we finish presenting an old frame
//     // Having knowledge of this helps us know how long the resources need to be kept around after the
//     // asset is released (the GPU may still be using it)
//     //
//
//     /// When called, the most recently committed asset version is "locked in" for the given frame
//     fn on_gpu_begin_read_for_frame(frame_index: u32);
//
//     /// When called, assets that were unloaded between begin(frame_index) and begin(frame_index+1)
//     /// are deallocated
//     fn on_gpu_end_read_for_frame(frame_index: u32);
//
//     //
//     // Calls from the asset system
//     //
//
//     /// Initiates uploading data for the given asset. When complete, load_op.complete() will be
//     /// called
//     fn update_asset(uuid: u32, data: Vec<u8>, version: u32);
//
//     /// Sets the given version as most recently committed, which means future calls to
//     /// on_gpu_begin_read_for_frame will pin the frame to the committed data
//     fn commit_asset_version(version: u32);
//
//     /// Queues resources to be released once the current frame ends (most recent index passed to
//     /// on_gpu_begin_read_for_frame). Future frames will not have access to this resource at all.
//     fn free(uuid: u32);
//
//     //
//     // Fetches data
//     //
//
//     /// Returns the GPU resource associated with the given frame
//     fn get_resource(uuid: u32, frame_index: u32) -> Vec<u8>;
// }

// - Complex version likely requires a hashmap per frame in flight
// - Complex version can fetch state of any frame - might be useful for runtime or debugging
// - If we have a hashmap per frame, we can throw them in arcs and pass them around. This potentially
//   decouples downstream users from this API and avoid them making their own copies of stuff
// - Using the simple version we can probably just keep a single hash map and lists of pending
//   changes to it. This is likely easier to implement.
// - The simple version assumes

/*

mod complex {
    use fnv::FnvHashMap;

    type FrameIndex = u32;
    type AssetUuid = u32;
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
        data: Vec<u8>
    }

    #[derive(Default)]
    struct TextureAssetUploader {
        pending_uploads: Vec<PendingUpload>,
        in_progress_uploads: Vec<InProgressUpload>,
        completed_uploads: Vec<CompletedUpload>,
        pending_removes: FnvHashMap<FrameIndex, Vec<AssetUuid>>,

        assets: FnvHashMap<AssetUuid, TextureHandle>,

        current_frame: u32,
        assets_for_frame: FnvHashMap<FrameIndex, FnvHashMap<AssetUuid, TextureHandle>>,
    }

    impl TextureAssetUploader {
        pub fn new() -> Self {
            Default::default()
        }

        //
        // These are called when we start preparing a new frame and when we finish presenting an old frame
        // Having knowledge of this helps us know how long the resources need to be kept around after the
        // asset is released (the GPU may still be using it)
        //

        // When called, the most recently committed asset version is "locked in" for the given frame
        fn on_gpu_begin_read_for_frame(&mut self, frame_index: u32) {
            // Lock in current asset state
            self.current_frame = frame_index;
            self.assets_for_frame.insert(frame_index, self.assets.clone());

        }

        // When called, assets that were unloaded between begin(frame_index) and begin(frame_index+1)
        // are deallocated
        fn on_gpu_end_read_for_frame(&mut self, frame_index: FrameIndex) {
            self.assets_for_frame.remove(&frame_index);

            // Drop everything in pending_unloads under frame_index out of the asset hashmap
            let assets_to_remove = self.pending_removes.get(&frame_index);
            if let Some(assets_to_remove) = assets_to_remove {
                for asset in assets_to_remove {
                    self.assets.remove(asset);
                }
            }

            self.pending_removes.remove(&frame_index);
        }

        //
        // Calls from the asset system
        //

        // Initiates uploading data for the given asset. When complete, load_op.complete() will be
        // called
        fn update_asset(&mut self, uuid: u32, data: Vec<u8>, version: u32) {
            // Push the data into pending_uploads.. either kick off a task to do the upload or
            // wait until later to kick it off as a batch

            self.pending_uploads.push(PendingUpload {
                uuid,
                data
            });
        }

        // Sets the given version as most recently committed, which means future calls to
        // on_gpu_begin_read_for_frame will pin the frame to the committed data
        //fn commit_asset_version(&self, version: u32);

        // Queues resources to be released once the current frame ends (most recent index passed to
        // on_gpu_begin_read_for_frame). Future frames will not have access to this resource at all.
        //fn free(&self, uuid: u32);

        //
        // Fetches data
        //

        // Returns the GPU resource associated with the given frame
        //fn get_resource(&self, uuid: u32, frame_index: u32) -> Vec<u8>;
    }
}



























mod simple {
    use fnv::FnvHashMap;

    type FrameIndex = u32;
    type AssetUuid = u32;
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

        /// When called, assets that were unloaded during the given frame are destroyed
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

        //
        // Calls from the asset system
        //

        /// Initiates uploading data for the given asset. When complete, load_op.complete() will be
        /// called
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


        //
        // Fetches data
        //

        /// Returns the GPU resource associated with the given frame.
        ///
        /// WARNING: Do not mix resources from before and after an asset loading tick.
        fn get_resource(&self, uuid: AssetUuid) -> Option<TextureHandle> {
            // Try to fetch it from assets
            self.assets.get(&uuid).map(|x| *x)
        }

        fn update(&mut self) {
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
    }
}
*/












use fnv::FnvHashMap;






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
