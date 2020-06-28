use ash::vk;
use renderer_assets::assets::pipeline::{
    PipelineAssetData, MaterialPassShaderInterface, MaterialAssetData, MaterialInstanceSlotAssignment,
    RenderpassAssetData,
};
use super::PipelineCreateData;
use fnv::FnvHashMap;
use renderer_shell_vulkan::{VkImageRaw, VkBufferRaw};
use super::DescriptorSetArc;
use atelier_assets::loader::LoadHandle;
use atelier_assets::loader::handle::Handle;
use std::sync::{Arc, Mutex};
use crate::resource_managers::resource_lookup::{
    DescriptorSetLayoutResource, PipelineLayoutResource, PipelineResource, ImageViewResource,
    ImageKey, BufferKey,
};
use crate::resource_managers::ResourceArc;
use super::DescriptorSetWriteSet;
use type_uuid::*;

//
// The "loaded" state of assets. Assets may have dependencies. Arcs to those dependencies ensure
// they do not get destroyed. All of the raw resources are hashed to avoid duplicating anything that
// is functionally identical. So for example if you have two windows with identical swapchain
// surfaces, they could share the same renderpass/pipeline resources
//
#[derive(TypeUuid, Clone)]
#[uuid = "b6958faa-5769-4048-a507-f91a07f49af4"]
pub struct ShaderAsset {
    pub shader_module: ResourceArc<vk::ShaderModule>,
}

// The actual GPU resources are held in Material because the pipeline does not specify everything
// needed to create the pipeline
#[derive(TypeUuid, Clone)]
#[uuid = "7a6a7ba8-a3ca-41eb-94f4-5d3723cd8b44"]
pub struct PipelineAsset {
    // We need to keep a copy of the asset so that we can recreate the pipeline for new swapchains
    pub pipeline_asset: Arc<PipelineAssetData>,
}

#[derive(TypeUuid, Clone)]
#[uuid = "bfefdc09-1ba6-422a-9514-b59b5b913128"]
pub struct RenderpassAsset {
    // We need to keep a copy of the asset so that we can recreate the pipeline for new swapchains
    pub data: Arc<RenderpassAssetData>,
}

pub struct SlotLocation {
    pub layout_index: u32,
    pub binding_index: u32,
    //pub array_index: u32,
}

pub type SlotNameLookup = FnvHashMap<String, Vec<SlotLocation>>;

pub struct PerSwapchainData {
    pub pipeline: ResourceArc<PipelineResource>,
}

//#[derive(TypeUuid)]
//#[uuid = "ec6b716d-64cb-452b-b973-1a6dcef58d2a"]
pub struct LoadedMaterialPass {
    pub shader_modules: Vec<ResourceArc<vk::ShaderModule>>,
    pub descriptor_set_layouts: Vec<ResourceArc<DescriptorSetLayoutResource>>,
    pub pipeline_layout: ResourceArc<PipelineLayoutResource>,

    // Potentially one of these per swapchain surface
    pub per_swapchain_data: Mutex<Vec<PerSwapchainData>>,

    // We need to keep a copy of the asset so that we can recreate the pipeline for new swapchains
    pub pipeline_create_data: PipelineCreateData,

    //descriptor_set_factory: DescriptorSetFactory,
    pub shader_interface: MaterialPassShaderInterface,

    //TODO: Use hash instead of string. Probably want to have a "hashed string" type that keeps the
    // string around only in debug mode. Maybe this could be generalized to a HashOfThing<T>.
    pub pass_slot_name_lookup: Arc<SlotNameLookup>,
}

#[derive(TypeUuid, Clone)]
#[uuid = "165673cd-d81d-4708-b9a4-d7e1a2a67976"]
pub struct MaterialAsset {
    pub passes: Arc<Vec<LoadedMaterialPass>>,
}

pub struct MaterialInstanceAssetInner {
    pub material: Handle<MaterialAssetData>,

    // Arc these individually because some downstream systems care only about the descriptor sets
    pub material_descriptor_sets: Arc<Vec<Vec<DescriptorSetArc>>>,
    pub slot_assignments: Vec<MaterialInstanceSlotAssignment>,
    pub descriptor_set_writes: Vec<Vec<DescriptorSetWriteSet>>,
}

#[derive(TypeUuid, Clone)]
#[uuid = "c60f6a3d-3e8d-4eea-8576-0971cd71b60f"]
pub struct MaterialInstanceAsset {
    pub inner: Arc<MaterialInstanceAssetInner>
}

#[derive(TypeUuid, Clone)]
#[uuid = "7a67b850-17f9-4877-8a6e-293a1589bbd8"]
pub struct ImageAsset {
    pub image_key: ImageKey,
    pub image: ResourceArc<VkImageRaw>,
    pub image_view: ResourceArc<ImageViewResource>,
}

#[derive(TypeUuid, Clone)]
#[uuid = "fc3b1eb8-c986-449e-a165-6a8f4582e6c5"]
pub struct BufferAsset {
    pub buffer_key: BufferKey,
    pub buffer: ResourceArc<VkBufferRaw>,
}

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
pub struct LoadedAssetLookupSet {
    pub shader_modules: AssetLookup<ShaderAsset>,
    pub graphics_pipelines: AssetLookup<PipelineAsset>,
    pub renderpasses: AssetLookup<RenderpassAsset>,
    pub materials: AssetLookup<MaterialAsset>,
    pub material_instances: AssetLookup<MaterialInstanceAsset>,
    pub images: AssetLookup<ImageAsset>,
    pub buffers: AssetLookup<BufferAsset>,
}

impl LoadedAssetLookupSet {
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
