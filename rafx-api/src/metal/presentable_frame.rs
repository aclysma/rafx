use crate::metal::RafxTextureMetal;
use crate::RafxError;

pub struct RafxPresentableFrameMetal {
    pub(crate) drawable: metal::CoreAnimationDrawable,

    // This is a cached wrapper around the texture provided by the drawable.
    pub(crate) texture: RafxTextureMetal,
}

impl RafxPresentableFrameMetal {
    pub fn new(drawable: metal::CoreAnimationDrawable) -> Self {
        let texture = RafxTextureMetal::new_from_metal_texture(drawable.texture().to_owned());
        RafxPresentableFrameMetal { drawable, texture }
    }

    pub fn cancel_present(
        self,
        _result: RafxError,
    ) {
        // just drop self
        //TODO: Send result to surface
    }

    pub fn present(self /* command buffers? */) {
        // TODO: wait for something?
        self.drawable.present();
    }

    pub fn texture_ref(&self) -> &RafxTextureMetal {
        // This increments the ref count on the texture
        &self.texture
    }

    pub fn texture(&self) -> RafxTextureMetal {
        // // This increments the ref count on the texture
        // &self.texture
        RafxTextureMetal::new_from_metal_texture(self.drawable.texture().to_owned())
    }
}

/*
impl FrameInFlight {
    // A value that stays in step with the image index returned by the swapchain. There is no
    // guarantee on the ordering of present image index (i.e. it may decrease). It is only promised
    // to not be in use by a frame in flight
    pub fn present_index(&self) -> u32;

    // If true, consider recreating the swapchain
    pub fn is_suboptimal(&self) -> bool;

    // Can be called by the end user to end the frame early and defer a result to the next acquire
    // image call
    pub fn cancel_present(
        self,
        result: VkResult<()>,
    );

    // submit the given command buffers and preset the swapchain image for this frame
    pub fn present(
        self,
        command_buffers: &[vk::CommandBuffer],
    ) -> VkResult<()>;
}
*/
