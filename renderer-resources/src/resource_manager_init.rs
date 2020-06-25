//use legion::prelude::*;
use renderer_shell_vulkan::VkDeviceContext;
use crate::resource_managers::ResourceManager;
use renderer_assets::asset_resource::AssetResource;
use renderer_assets::assets::shader::ShaderAsset;
use renderer_assets::assets::pipeline::{
    PipelineAsset, MaterialAsset, MaterialInstanceAsset, RenderpassAsset,
};
use renderer_assets::assets::image::ImageAsset;
use renderer_assets::assets::buffer::BufferAsset;
use ash::prelude::VkResult;

pub fn create_resource_manager(
    device_context: &VkDeviceContext,
    asset_resource: &mut AssetResource,
) -> ResourceManager {
    let mut resource_manager = ResourceManager::new(&device_context);

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

    resource_manager
}
