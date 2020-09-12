use fnv::FnvHashMap;
use atelier_assets::loader::LoadHandle;
use crate::{
    ShaderAsset, PipelineAsset, RenderpassAsset, MaterialAsset, MaterialInstanceAsset, ImageAsset,
    BufferAsset,
};

//
// Represents a single asset which may simultaneously have committed and uncommitted loaded state
//
pub struct LoadedAssetState<AssetT> {
    pub committed: Option<AssetT>,
    pub uncommitted: Option<AssetT>,
}

impl<AssetT> Default for LoadedAssetState<AssetT> {
    fn default() -> Self {
        LoadedAssetState {
            committed: None,
            uncommitted: None,
        }
    }
}

pub struct AssetLookup<AssetT> {
    //TODO: Slab these for faster lookup?
    pub loaded_assets: FnvHashMap<LoadHandle, LoadedAssetState<AssetT>>,
}

impl<AssetT> AssetLookup<AssetT> {
    pub fn set_uncommitted(
        &mut self,
        load_handle: LoadHandle,
        loaded_asset: AssetT,
    ) {
        self.loaded_assets
            .entry(load_handle)
            .or_default()
            .uncommitted = Some(loaded_asset);
    }

    pub fn commit(
        &mut self,
        load_handle: LoadHandle,
    ) {
        let state = self.loaded_assets.get_mut(&load_handle).unwrap();
        state.committed = state.uncommitted.take();
    }

    pub fn free(
        &mut self,
        load_handle: LoadHandle,
    ) {
        let old = self.loaded_assets.remove(&load_handle);
        assert!(old.is_some());
    }

    pub fn get_latest(
        &self,
        load_handle: LoadHandle,
    ) -> Option<&AssetT> {
        if let Some(loaded_assets) = self.loaded_assets.get(&load_handle) {
            if let Some(uncommitted) = &loaded_assets.uncommitted {
                Some(uncommitted)
            } else if let Some(committed) = &loaded_assets.committed {
                Some(committed)
            } else {
                // It's an error to reach here because of uncommitted and committed are none, there
                // shouldn't be an entry in loaded_assets
                unreachable!();
            }
        } else {
            None
        }
    }

    pub fn get_committed(
        &self,
        load_handle: LoadHandle,
    ) -> Option<&AssetT> {
        if let Some(loaded_assets) = self.loaded_assets.get(&load_handle) {
            if let Some(committed) = &loaded_assets.committed {
                Some(committed)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.loaded_assets.len()
    }

    pub fn is_empty(&self) -> bool { self.loaded_assets.is_empty() }

    pub fn destroy(&mut self) {
        self.loaded_assets.clear();
    }
}

impl<AssetT> Default for AssetLookup<AssetT> {
    fn default() -> Self {
        AssetLookup {
            loaded_assets: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct LoadedAssetMetrics {
    pub shader_module_count: usize,
    pub pipeline_count: usize,
    pub renderpass_count: usize,
    pub material_count: usize,
    pub material_instance_count: usize,
    pub image_count: usize,
    pub buffer_count: usize,
}

//
// Lookups by asset for loaded asset state
//
#[derive(Default)]
pub struct AssetLookupSet {
    pub shader_modules: AssetLookup<ShaderAsset>,
    pub graphics_pipelines: AssetLookup<PipelineAsset>,
    pub renderpasses: AssetLookup<RenderpassAsset>,
    pub materials: AssetLookup<MaterialAsset>,
    pub material_instances: AssetLookup<MaterialInstanceAsset>,
    pub images: AssetLookup<ImageAsset>,
    pub buffers: AssetLookup<BufferAsset>,
}

impl AssetLookupSet {
    pub fn metrics(&self) -> LoadedAssetMetrics {
        LoadedAssetMetrics {
            shader_module_count: self.shader_modules.len(),
            pipeline_count: self.graphics_pipelines.len(),
            renderpass_count: self.renderpasses.len(),
            material_count: self.materials.len(),
            material_instance_count: self.material_instances.len(),
            image_count: self.images.len(),
            buffer_count: self.buffers.len(),
        }
    }

    pub fn destroy(&mut self) {
        self.shader_modules.destroy();
        self.graphics_pipelines.destroy();
        self.renderpasses.destroy();
        self.materials.destroy();
        self.material_instances.destroy();
        self.images.destroy();
        self.buffers.destroy();
    }
}
