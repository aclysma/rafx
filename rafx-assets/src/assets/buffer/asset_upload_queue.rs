use crate::assets::load_queue::LoadRequest;
use crate::assets::upload_asset_op::{UploadAssetOp, UploadAssetOpResult};
use crate::{BufferAsset, BufferAssetData};
use crossbeam_channel::{Receiver, Sender};
use rafx_api::RafxBuffer;
use rafx_framework::upload::UploadQueueContext;
use rafx_framework::RafxResult;

pub type BufferAssetUploadOpResult = UploadAssetOpResult<RafxBuffer, BufferAsset>;

pub struct BufferAssetUploadQueue {
    pub upload_queue_context: UploadQueueContext,

    pub buffer_upload_result_tx: Sender<BufferAssetUploadOpResult>,
    pub buffer_upload_result_rx: Receiver<BufferAssetUploadOpResult>,
}

impl BufferAssetUploadQueue {
    pub fn new(upload_queue_context: UploadQueueContext) -> RafxResult<Self> {
        let (buffer_upload_result_tx, buffer_upload_result_rx) = crossbeam_channel::unbounded();

        Ok(BufferAssetUploadQueue {
            upload_queue_context,
            buffer_upload_result_tx,
            buffer_upload_result_rx,
        })
    }

    pub fn upload_buffer(
        &self,
        request: LoadRequest<BufferAssetData, BufferAsset>,
    ) -> RafxResult<()> {
        let op = Box::new(UploadAssetOp::new(
            request.load_op,
            request.load_handle,
            request.result_tx,
            self.buffer_upload_result_tx.clone(),
        ));
        assert!(!request.asset.data.is_empty());
        self.upload_queue_context.upload_new_buffer(
            op,
            request.asset.resource_type,
            request.asset.data,
        )
    }
}
