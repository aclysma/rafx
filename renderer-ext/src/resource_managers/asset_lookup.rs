use super::resource_lookup::ResourceArc;
use ash::vk;
use crate::pipeline::pipeline::{
    PipelineAsset, MaterialPassShaderInterface, MaterialAsset, MaterialInstanceSlotAssignment,
};
use super::PipelineCreateData;
use fnv::FnvHashMap;
use renderer_shell_vulkan::{VkImageRaw, VkBufferRaw};
use super::DescriptorSetArc;
use atelier_assets::loader::LoadHandle;
use atelier_assets::loader::handle::Handle;
use std::sync::Arc;
use crate::resource_managers::resource_lookup::{DescriptorSetLayoutResource, PipelineLayoutResource, PipelineResource, ImageViewResource, ImageKey, BufferKey};
use crate::pipeline::gltf::MeshAsset;

//
// The "loaded" state of assets. Assets may have dependencies. Arcs to those dependencies ensure
// they do not get destroyed. All of the raw resources are hashed to avoid duplicating anything that
// is functionally identical. So for example if you have two windows with identical swapchain
// surfaces, they could share the same renderpass/pipeline resources
//
pub struct LoadedShaderModule {
    pub shader_module: ResourceArc<vk::ShaderModule>,
}

// The actual GPU resources are held in Material because the pipeline does not specify everything
// needed to create the pipeline
pub struct LoadedGraphicsPipeline {
    // We need to keep a copy of the asset so that we can recreate the pipeline for new swapchains
    pub pipeline_asset: PipelineAsset,
}

pub struct SlotLocation {
    pub layout_index: u32,
    pub binding_index: u32,
    //pub array_index: u32,
}

pub type SlotNameLookup = FnvHashMap<String, Vec<SlotLocation>>;

pub struct LoadedMaterialPass {
    pub shader_modules: Vec<ResourceArc<vk::ShaderModule>>,
    pub descriptor_set_layouts: Vec<ResourceArc<DescriptorSetLayoutResource>>,
    pub pipeline_layout: ResourceArc<PipelineLayoutResource>,

    // Potentially one of these per swapchain surface
    pub render_passes: Vec<ResourceArc<vk::RenderPass>>,
    pub pipelines: Vec<ResourceArc<PipelineResource>>,

    // We need to keep a copy of the asset so that we can recreate the pipeline for new swapchains
    pub pipeline_create_data: PipelineCreateData,

    //descriptor_set_factory: DescriptorSetFactory,
    pub shader_interface: MaterialPassShaderInterface,

    //TODO: Use hash instead of string. Probably want to have a "hashed string" type that keeps the
    // string around only in debug mode. Maybe this could be generalized to a HashOfThing<T>.
    pub pass_slot_name_lookup: Arc<SlotNameLookup>,
}

pub struct LoadedMaterial {
    pub passes: Vec<LoadedMaterialPass>,
}

pub struct LoadedMaterialInstance {
    pub material: Handle<MaterialAsset>,
    pub material_descriptor_sets: Vec<Vec<DescriptorSetArc>>,
    pub slot_assignments: Vec<MaterialInstanceSlotAssignment>,
}

pub struct LoadedImage {
    //image_load_handle: LoadHandle,
    //image_view_meta: dsc::ImageViewMeta,
    pub image_key: ImageKey,
    pub image: ResourceArc<VkImageRaw>,
    pub image_view: ResourceArc<ImageViewResource>,
    // One per swapchain
    //image_views: Vec<ResourceArc<ImageViewResource>>
}

pub struct LoadedBuffer {
    pub buffer_key: BufferKey,
    pub buffer: ResourceArc<VkBufferRaw>,
}

pub struct LoadedMesh {
    pub vertex_buffer: ResourceArc<VkBufferRaw>,
    pub index_buffer: ResourceArc<VkBufferRaw>,
    pub asset: MeshAsset,
}

//
// Represents a single asset which may simultaneously have committed and uncommitted loaded state
//
pub struct LoadedAssetState<LoadedAssetT> {
    pub committed: Option<LoadedAssetT>,
    pub uncommitted: Option<LoadedAssetT>,
}

impl<LoadedAssetT> Default for LoadedAssetState<LoadedAssetT> {
    fn default() -> Self {
        LoadedAssetState {
            committed: None,
            uncommitted: None,
        }
    }
}

pub struct AssetLookup<LoadedAssetT> {
    //TODO: Slab these for faster lookup?
    pub loaded_assets: FnvHashMap<LoadHandle, LoadedAssetState<LoadedAssetT>>,
}

impl<LoadedAssetT> AssetLookup<LoadedAssetT> {
    pub fn set_uncommitted(
        &mut self,
        load_handle: LoadHandle,
        loaded_asset: LoadedAssetT,
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
    ) -> Option<&LoadedAssetT> {
        if let Some(loaded_assets) = self.loaded_assets.get(&load_handle) {
            if let Some(uncommitted) = &loaded_assets.uncommitted {
                Some(uncommitted)
            } else if let Some(committed) = &loaded_assets.committed {
                Some(committed)
            } else {
                // It's an error to reach here because of uncommitted and committed are none, there
                // shouldn't be an entry in loaded_assets
                unreachable!();
                None
            }
        } else {
            None
        }
    }

    pub fn get_committed(
        &self,
        load_handle: LoadHandle,
    ) -> Option<&LoadedAssetT> {
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

    fn destroy(&mut self) {
        self.loaded_assets.clear();
    }
}

impl<LoadedAssetT> Default for AssetLookup<LoadedAssetT> {
    fn default() -> Self {
        AssetLookup {
            loaded_assets: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct LoadedAssetMetrics {
    shader_module_count: usize,
    pipeline_count: usize,
    material_count: usize,
    material_instance_count: usize,
    image_count: usize,
    buffer_count: usize,
    mesh_count: usize,
}

//
// Lookups by asset for loaded asset state
//
#[derive(Default)]
pub struct LoadedAssetLookupSet {
    pub shader_modules: AssetLookup<LoadedShaderModule>,
    pub graphics_pipelines2: AssetLookup<LoadedGraphicsPipeline>,
    pub materials: AssetLookup<LoadedMaterial>,
    pub material_instances: AssetLookup<LoadedMaterialInstance>,
    pub images: AssetLookup<LoadedImage>,
    pub buffers: AssetLookup<LoadedBuffer>,
    pub meshes: AssetLookup<LoadedMesh>,
}

impl LoadedAssetLookupSet {
    pub fn metrics(&self) -> LoadedAssetMetrics {
        LoadedAssetMetrics {
            shader_module_count: self.shader_modules.len(),
            pipeline_count: self.graphics_pipelines2.len(),
            material_count: self.materials.len(),
            material_instance_count: self.material_instances.len(),
            image_count: self.images.len(),
            buffer_count: self.buffers.len(),
            mesh_count: self.meshes.len(),
        }
    }

    pub fn destroy(&mut self) {
        self.shader_modules.destroy();
        self.graphics_pipelines2.destroy();
        self.materials.destroy();
        self.material_instances.destroy();
        self.images.destroy();
        self.buffers.destroy();
        self.meshes.destroy();
    }
}
