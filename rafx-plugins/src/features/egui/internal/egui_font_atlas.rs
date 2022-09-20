use rafx::api::{
    RafxBufferDef, RafxExtents3D, RafxFormat, RafxResourceType, RafxResult, RafxTextureDef,
};
use rafx::framework::{BufferResource, DynResourceAllocatorSet, ImageViewResource, ResourceArc};
use std::sync::Arc;

pub struct EguiImageUpdate {
    pub upload_buffer: ResourceArc<BufferResource>,
    pub upload_image: ResourceArc<ImageViewResource>,
}

#[derive(Clone, Default)]
pub struct EguiFontAtlasCache {
    pub image_resource: Option<ResourceArc<ImageViewResource>>,
    pub font_atlas: Option<Arc<egui::Texture>>,
}

impl EguiFontAtlasCache {
    pub fn font_atlas_resource(&self) -> &Option<ResourceArc<ImageViewResource>> {
        &self.image_resource
    }

    pub fn update(
        &mut self,
        dyn_resource_allocator: &DynResourceAllocatorSet,
        font_atlas: &Arc<egui::Texture>,
    ) -> RafxResult<Option<EguiImageUpdate>> {
        if Some(font_atlas.version) != self.font_atlas.as_ref().map(|x| x.version) {
            self.font_atlas = Some(font_atlas.clone());
            let extents = RafxExtents3D {
                width: font_atlas.width as u32,
                height: font_atlas.height as u32,
                depth: 1,
            };

            let buffer = dyn_resource_allocator.device_context.create_buffer(
                &RafxBufferDef::for_staging_buffer_data(
                    &font_atlas.pixels,
                    RafxResourceType::BUFFER,
                ),
            )?;
            buffer
                .copy_to_host_visible_buffer(&font_atlas.pixels)
                .unwrap();
            let buffer = dyn_resource_allocator.insert_buffer(buffer);

            // No mips, egui generates text exactly-sized for the screen
            let mip_count = 1;
            let texture =
                dyn_resource_allocator
                    .device_context
                    .create_texture(&RafxTextureDef {
                        extents,
                        // Ideally this would be an SRGB texture, but VK_FORMAT_FEATURE_BLIT_DST_BIT is not available for SRGB textures on all hardware
                        format: RafxFormat::R8_UNORM,
                        mip_count,
                        ..Default::default()
                    })?;
            texture.set_debug_name("egui Font Atlas");

            let image = dyn_resource_allocator.insert_texture(texture);
            let image_view = dyn_resource_allocator.insert_image_view(&image, None)?;

            self.image_resource = Some(image_view.clone());

            Ok(Some(EguiImageUpdate {
                upload_buffer: buffer,
                upload_image: image_view,
            }))
        } else {
            Ok(None)
        }
    }
}
