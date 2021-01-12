// pub struct RafxTexturePtrMetal {
//     texture: metal::TextureRef,
// }
//
// impl RafxTexturePtrMetal {
//     pub fn texture(&self) -> *metal::TextureRef {
//         &self.texture
//     }
// }

//#[derive(Clone)]
#[derive(Debug)]
pub struct RafxTextureMetal {
    texture: metal::Texture,
}

unsafe impl Send for RafxTextureMetal {}
unsafe impl Sync for RafxTextureMetal {}

impl RafxTextureMetal {
    pub fn new_from_metal_texture(texture: metal::Texture) -> Self {
        RafxTextureMetal { texture }
    }

    pub fn texture(&self) -> &metal::Texture {
        &self.texture
    }

    // pub fn take_texture_ptr(self) -> RafxTexturePtrMetal {
    //     RafxTexturePtrMetal {
    //
    //     }
    // }
}
