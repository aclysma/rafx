pub mod conversions;
pub mod features;
pub mod util;

bitflags::bitflags! {
    pub struct BarrierFlagsMetal: u8 {
        const BUFFERS = 1;
        const TEXTURES = 2;
        const RENDER_TARGETS = 4;
        const FENCE = 8;
    }
}
