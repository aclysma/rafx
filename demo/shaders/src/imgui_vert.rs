// This code is auto-generated by the shader processor.

#[allow(unused_imports)]
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
#[repr(C)]
pub struct ArgsStd140 {
    pub mvp: [[f32; 4]; 4], // +0 (size: 64)
} // 64 bytes

pub type ArgsUniform = ArgsStd140;

pub const UNIFORM_BUFFER_DESCRIPTOR_SET_INDEX: usize = 0;
pub const UNIFORM_BUFFER_DESCRIPTOR_BINDING_INDEX: usize = 0;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_struct_args_std140() {
        assert_eq!(std::mem::size_of::<ArgsStd140>(), 64);
        assert_eq!(std::mem::size_of::<[[f32; 4]; 4]>(), 64);
        assert_eq!(std::mem::align_of::<[[f32; 4]; 4]>(), 4);
        assert_eq!(memoffset::offset_of!(ArgsStd140, mvp), 0);
    }
}
