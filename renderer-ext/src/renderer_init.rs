use legion::prelude::*;
use renderer_base::{RenderRegistryBuilder, RenderRegistry};
use crate::features::sprite::SpriteRenderFeature;
use crate::phases::draw_opaque::DrawOpaqueRenderPhase;
use crate::phases::draw_transparent::DrawTransparentRenderPhase;
use renderer_shell_vulkan::VkDeviceContext;
use crate::resource_managers::ResourceManager;
use crate::asset_resource::AssetResource;
use crate::pipeline::shader::ShaderAsset;
use crate::pipeline::pipeline::{PipelineAsset, MaterialAsset, MaterialInstanceAsset, RenderpassAsset};
use crate::pipeline::image::ImageAsset;
use crate::pipeline::buffer::BufferAsset;
use crate::pipeline::gltf::{MeshAsset, GltfMaterialAsset};
use ash::prelude::VkResult;
use crate::features::mesh::MeshRenderFeature;
use crate::renderpass::debug_renderpass::DebugDraw3DResource;

pub fn init_renderer(
    resources: &mut Resources,
) {
    //
    // Register features/phases
    //
    let render_registry = RenderRegistryBuilder::default()
        .register_feature::<SpriteRenderFeature>()
        .register_feature::<MeshRenderFeature>()
        .register_render_phase::<DrawOpaqueRenderPhase>()
        .register_render_phase::<DrawTransparentRenderPhase>()
        .build();
    resources.insert(render_registry);
    resources.insert(DebugDraw3DResource::new());

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
    asset_resource.add_storage_with_load_handler::<RenderpassAsset, _>(Box::new(
        resource_manager.create_renderpass_load_handler(),
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
    resources.remove::<DebugDraw3DResource>();
}


