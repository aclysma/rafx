use hydrate_base::handle::Handle;
use rafx::assets::ImageAsset;

#[derive(Default)]
pub struct SkyboxResource {
    pub(super) skybox_texture: Option<Handle<ImageAsset>>,
}

impl SkyboxResource {
    pub fn skybox_texture(&self) -> &Option<Handle<ImageAsset>> {
        &self.skybox_texture
    }

    pub fn skybox_texture_mut(&mut self) -> &mut Option<Handle<ImageAsset>> {
        &mut self.skybox_texture
    }
}
