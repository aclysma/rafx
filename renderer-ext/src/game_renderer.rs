use crate::imgui_support::{VkImGuiRenderPassFontAtlas, VkImGuiRenderPass, ImguiRenderEventListener, Sdl2ImguiManager};
use renderer_shell_vulkan::{VkDevice, VkSwapchain, VkSurface, Window, VkTransferUpload, VkTransferUploadState, VkImage, VkDeviceContext, VkContextBuilder, VkCreateContextError, VkContext, VkSurfaceSwapchainLifetimeListener, MsaaLevel, MAX_FRAMES_IN_FLIGHT, VkBuffer};
use ash::prelude::VkResult;
use crate::renderpass::{VkSpriteRenderPass, VkMeshRenderPass, StaticMeshInstance, PerFrameDataShaderParam, PerObjectDataShaderParam, VkDebugRenderPass, VkBloomRenderPassResources, VkOpaqueRenderPass};
use std::mem::{ManuallyDrop, swap};
use crate::image_utils::{decode_texture, enqueue_load_images};
use ash::vk;
use crate::time::{ScopeTimer, TimeState};
use crossbeam_channel::Sender;
use std::ops::Deref;
// use crate::resource_managers::{
//     SpriteResourceManager, VkMeshResourceManager, ImageResourceManager,
//     MaterialResourceManager,
// };
//use crate::renderpass::VkMeshRenderPass;
use crate::pipeline_description::SwapchainSurfaceInfo;
use crate::pipeline::pipeline::{MaterialAsset, PipelineAsset, MaterialInstanceAsset};
use atelier_assets::loader::handle::Handle;
use crate::asset_resource::AssetResource;
//use crate::upload::UploadQueue;
//use crate::load_handlers::{ImageLoadHandler, MeshLoadHandler, SpriteLoadHandler, MaterialLoadHandler};
use crate::pipeline::shader::ShaderAsset;
use crate::pipeline::image::ImageAsset;
//use crate::pipeline::gltf::{GltfMaterialAsset, MeshAsset};
//use crate::pipeline::sprite::SpriteAsset;
use atelier_assets::core::asset_uuid;
use atelier_assets::loader::LoadStatus;
use atelier_assets::loader::handle::AssetHandle;
use atelier_assets::core as atelier_core;
use atelier_assets::core::AssetUuid;
use crate::resource_managers::{ResourceManager, DynDescriptorSet, DynMaterialInstance, MeshInfo};
use crate::pipeline::gltf::{MeshAsset, GltfMaterialAsset, GltfMaterialData, GltfMaterialDataShaderParam};
use crate::pipeline::buffer::BufferAsset;
use crate::renderpass::debug_renderpass::DebugDraw3DResource;
use crate::renderpass::VkBloomExtractRenderPass;
use crate::renderpass::VkBloomBlurRenderPass;
use crate::renderpass::VkBloomCombineRenderPass;
use crate::features::sprite::{SpriteRenderNodeSet, SpriteRenderFeature, create_sprite_extract_job};
use renderer_base::visibility::{StaticVisibilityNodeSet, DynamicVisibilityNodeSet};
use renderer_base::{RenderRegistryBuilder, RenderPhaseMaskBuilder, RenderPhaseMask, RenderRegistry, RenderViewSet, AllRenderNodes, FramePacketBuilder, ExtractJobSet};
use crate::phases::draw_opaque::DrawOpaqueRenderPhase;
use crate::phases::draw_transparent::DrawTransparentRenderPhase;
use legion::prelude::*;
use crate::{RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContextFactory};
use crate::RenderJobWriteContext;
use renderer_shell_vulkan::cleanup::{VkCombinedDropSink, VkResourceDropSinkChannel};




// pub struct GameRendererSystems {
//     render_registry: RenderRegistry,
//     resource_manager: ResourceManager,
// }

pub fn init_renderer(
    resources: &mut Resources,
) {
    //
    // Register features/phases
    //
    let render_registry = RenderRegistryBuilder::default()
        .register_feature::<SpriteRenderFeature>()
        .register_render_phase::<DrawOpaqueRenderPhase>()
        .register_render_phase::<DrawTransparentRenderPhase>()
        .build();
    resources.insert(render_registry);

    //
    // Create the resource manager
    //
    let device_context = resources.get_mut::<VkDeviceContext>().unwrap().clone();
    let mut resource_manager = ResourceManager::new(&device_context);
    resources.insert(resource_manager);

    //
    // Connect the asset system with the resource manager
    //
    let mut asset_resource_fetch = resources.get_mut::<AssetResource>().unwrap();
    let asset_resource = &mut *asset_resource_fetch;

    let mut resource_manager_fetch = resources.get_mut::<ResourceManager>().unwrap();
    let resource_manager = &mut *resource_manager_fetch;

    asset_resource.add_storage_with_load_handler::<ShaderAsset, _>(Box::new(
        resource_manager.create_shader_load_handler(),
    ));
    asset_resource.add_storage_with_load_handler::<PipelineAsset, _>(Box::new(
        resource_manager.create_pipeline_load_handler(),
    ));
    asset_resource.add_storage_with_load_handler::<MaterialAsset, _>(Box::new(
        resource_manager.create_material_load_handler(),
    ));
    asset_resource.add_storage_with_load_handler::<MaterialInstanceAsset, _>(Box::new(
        resource_manager.create_material_instance_load_handler(),
    ));
    asset_resource.add_storage_with_load_handler::<ImageAsset, _>(Box::new(
        resource_manager.create_image_load_handler(),
    ));
    asset_resource.add_storage_with_load_handler::<BufferAsset, _>(Box::new(
        resource_manager.create_buffer_load_handler(),
    ));
    asset_resource.add_storage_with_load_handler::<MeshAsset, _>(Box::new(
        resource_manager.create_mesh_load_handler(),
    ));

    asset_resource.add_storage::<GltfMaterialAsset>();
}

pub fn update_renderer(
    resources: &Resources
) -> VkResult<()> {
    resources.get_mut::<ResourceManager>().unwrap().update_resources()
}

pub fn destroy_renderer(
    resources: &mut Resources,
) {
    resources.remove::<ResourceManager>();
    resources.remove::<RenderRegistry>();
}


