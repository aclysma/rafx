use rafx::api::extra::upload::RafxTransferUpload;
use rafx::api::{RafxDeviceContext, RafxError, RafxResult};
use rafx::assets::image_upload::ImageUploadParams;
use rafx::assets::{image_upload, GpuImageData, GpuImageDataColorSpace};
use rafx::framework::{DynResourceAllocatorSet, ImageViewResource, ResourceArc};

pub struct ImguiFontAtlasData {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl ImguiFontAtlasData {
    pub fn new(texture: &imgui::FontAtlasTexture) -> Self {
        ImguiFontAtlasData {
            width: texture.width,
            height: texture.height,
            data: texture.data.to_vec(),
        }
    }
}

#[derive(Clone)]
pub struct ImguiFontAtlas(pub ResourceArc<ImageViewResource>);

#[cfg(feature = "use-imgui")]
pub(super) fn create_font_atlas_image_view(
    imgui_font_atlas_data: ImguiFontAtlasData,
    device_context: &RafxDeviceContext,
    upload: &mut RafxTransferUpload,
    dyn_resource_allocator: &DynResourceAllocatorSet,
) -> RafxResult<ResourceArc<ImageViewResource>> {
    let image_data = GpuImageData::new_simple(
        imgui_font_atlas_data.width,
        imgui_font_atlas_data.height,
        GpuImageDataColorSpace::Linear.rgba8(),
        imgui_font_atlas_data.data,
    );

    let texture = image_upload::enqueue_load_image(
        device_context,
        upload,
        &image_data,
        ImageUploadParams {
            generate_mips: false,
            ..Default::default()
        },
    )
    .map_err(|x| Into::<RafxError>::into(x))?;

    let image = dyn_resource_allocator.insert_texture(texture);

    Ok(dyn_resource_allocator.insert_image_view(&image, None)?)
}
