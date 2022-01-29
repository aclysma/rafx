use crossbeam_channel::Sender;
use distill::loader::storage::AssetLoadOp;
use distill::loader::LoadHandle;
use rafx_api::RafxError;
use rafx_framework::upload::UploadOp;

//
// Ghetto futures - UploadOp is used to signal completion and UploadOpAwaiter is used to check the result
//
pub enum UploadAssetOpResult<ResourceT, AssetT> {
    UploadError(LoadHandle),
    UploadComplete(AssetLoadOp, Sender<AssetT>, ResourceT),
    UploadDrop(LoadHandle),
}

pub struct UploadAssetOpInner<ResourceT, AssetT> {
    load_op: AssetLoadOp,
    load_handle: LoadHandle,
    asset_sender: Sender<AssetT>, // This sends back to the asset storage, we just pass it along
    sender: Sender<UploadAssetOpResult<ResourceT, AssetT>>, // This sends back to the resource manager to finalize the load
}

pub struct UploadAssetOp<ResourceT, AssetT> {
    inner: Option<UploadAssetOpInner<ResourceT, AssetT>>,
}

impl<ResourceT, AssetT> Drop for UploadAssetOp<ResourceT, AssetT> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            let sender = inner.sender;
            let _ = sender.send(UploadAssetOpResult::UploadDrop(inner.load_handle));
        }
    }
}

impl<ResourceT, AssetT> UploadAssetOp<ResourceT, AssetT> {
    pub fn new(
        load_op: AssetLoadOp,
        load_handle: LoadHandle,
        asset_sender: Sender<AssetT>,
        sender: Sender<UploadAssetOpResult<ResourceT, AssetT>>,
    ) -> Self {
        let inner = UploadAssetOpInner {
            load_op,
            load_handle,
            asset_sender,
            sender,
        };

        UploadAssetOp { inner: Some(inner) }
    }

    pub fn complete(
        mut self,
        resource: ResourceT,
    ) {
        self.do_complete(resource)
    }

    pub fn error(
        mut self: Box<Self>,
        error: RafxError,
    ) {
        self.do_error(error)
    }

    fn do_complete(
        &mut self,
        resource: ResourceT,
    ) {
        let inner = self.inner.take().unwrap();
        let load_op = inner.load_op;
        let asset_sender = inner.asset_sender;
        let _ = inner.sender.send(UploadAssetOpResult::UploadComplete(
            load_op,
            asset_sender,
            resource,
        ));
    }

    fn do_error(
        &mut self,
        error: RafxError,
    ) {
        let inner = self.inner.take().unwrap();
        inner.load_op.error(error);
        let _ = inner
            .sender
            .send(UploadAssetOpResult::UploadError(inner.load_handle));
    }
}

impl<ResourceT: Send + Sync, AssetT: Send + Sync> UploadOp<ResourceT>
    for UploadAssetOp<ResourceT, AssetT>
{
    fn complete(
        mut self: Box<Self>,
        resource: ResourceT,
    ) {
        self.do_complete(resource)
    }

    fn error(
        mut self: Box<Self>,
        error: RafxError,
    ) {
        self.do_error(error)
    }
}
