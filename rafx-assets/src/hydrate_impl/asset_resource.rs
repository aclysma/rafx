// use distill::loader::{handle::RefOp, Loader};
//
// use super::asset_storage::AssetStorageSet;
// use super::asset_storage::DynAssetLoader;
//
// use type_uuid::TypeUuid;
//
// use crossbeam_channel::{Receiver, Sender};
// use hydrate_base::handle::AssetHandle;
// use hydrate_base::handle::Handle;
// use hydrate_base::AssetUuid;
// use hydrate_loader::storage::IndirectIdentifier;
// use hydrate_loader::storage::IndirectionResolver;
// use hydrate_loader::storage::LoadInfo;
// use hydrate_loader::LoadState;
//
// /// A user-friendly interface to fetching/storing/loading assets. Meant to be a resource in an ECS
// /// system
// pub struct AssetResource {
//     loader: Loader,
//     resolver: Box<dyn IndirectionResolver + Send + Sync + 'static>,
//     storage: AssetStorageSet,
//     tx: Sender<RefOp>,
//     rx: Receiver<RefOp>,
// }
//
// impl AssetResource {
//     pub fn new(
//         loader: Loader,
//         resolver: Box<dyn IndirectionResolver + Send + Sync + 'static>,
//     ) -> Self {
//         let (tx, rx) = crossbeam_channel::unbounded();
//         let storage = AssetStorageSet::new(tx.clone(), loader.indirection_table());
//
//         AssetResource {
//             loader,
//             resolver,
//             storage,
//             tx,
//             rx,
//         }
//     }
// }
//
// impl AssetResource {
//     /// Adds a default storage object for assets of type T
//     pub fn add_storage<T: TypeUuid + for<'a> serde::Deserialize<'a> + 'static + Send>(&mut self) {
//         self.storage.add_storage::<T>();
//     }
//
//     /// Adds a storage object for assets of type T that proxies loading events to the given loader.
//     /// This allows an end-user to do additional processing to "prepare" the asset. For example, a
//     /// texture might be uploaded to GPU memory before being considered loaded.
//     pub fn add_storage_with_loader<AssetDataT, AssetT, LoaderT>(
//         &mut self,
//         loader: Box<LoaderT>,
//     ) where
//         AssetDataT: TypeUuid + for<'a> serde::Deserialize<'a> + 'static,
//         AssetT: TypeUuid + 'static + Send,
//         LoaderT: DynAssetLoader<AssetT> + 'static,
//     {
//         self.storage
//             .add_storage_with_loader::<AssetDataT, AssetT, LoaderT>(loader);
//     }
//
//     pub fn loader(&self) -> &Loader {
//         &self.loader
//     }
//
//     /// Call this frequently to update the asset loading system.
//     #[profiling::function]
//     pub fn update(&mut self) {
//         hydrate_loader::process_ref_ops(&self.loader, &self.rx);
//         self.loader
//             .process(&mut self.storage, &*self.resolver)
//             .expect("failed to process loader");
//     }
//
//     //
//     // These functions map to distill APIs
//     //
//     pub fn load_asset<T>(
//         &self,
//         asset_uuid: AssetUuid,
//     ) -> Handle<T> {
//         let load_handle = self.loader.add_ref(asset_uuid);
//         Handle::<T>::new(self.tx.clone(), load_handle)
//     }
//
//     pub fn load_asset_indirect<T>(
//         &self,
//         id: IndirectIdentifier,
//     ) -> Handle<T> {
//         let load_handle = self.loader.add_ref_indirect(id);
//         Handle::<T>::new(self.tx.clone(), load_handle)
//     }
//
//     pub fn load_asset_path<T: TypeUuid + 'static + Send, U: Into<String>>(
//         &self,
//         path: U,
//     ) -> Handle<T> {
//         let data_type_uuid = self
//             .storage
//             .asset_to_data_type_uuid::<T>()
//             .expect("Called load_asset_path with unregistered asset type");
//
//         let load_handle = self
//             .loader
//             .add_ref_indirect(IndirectIdentifier::PathWithType(
//                 path.into(),
//                 data_type_uuid,
//             ));
//         Handle::<T>::new(self.tx.clone(), load_handle)
//     }
//
//     pub fn asset<T: TypeUuid + 'static + Send>(
//         &self,
//         handle: &Handle<T>,
//     ) -> Option<&T> {
//         handle.asset(&self.storage)
//     }
//
//     pub fn asset_version<T: TypeUuid + 'static + Send>(
//         &self,
//         handle: &Handle<T>,
//     ) -> Option<u32> {
//         handle.asset_version::<T, _>(&self.storage)
//     }
//
//     pub fn load_status<T>(
//         &self,
//         handle: &Handle<T>,
//     ) -> LoadState {
//         handle.load_status(&self.loader)
//     }
//
//     // pub fn load_info<T>(
//     //     &self,
//     //     handle: &Handle<T>,
//     // ) -> Option<LoadState> {
//     //     self.loader.get_load_info(handle.load_handle())
//     // }
// }

pub use hydrate_loader::AssetManager as AssetResource;
