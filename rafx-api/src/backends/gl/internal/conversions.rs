use crate::RafxMemoryUsage;
use crate::gl::gles20::types::GLenum;
use crate::gl::gles20;

impl RafxMemoryUsage {
    pub fn gl_usage(self) -> Option<GLenum> {
        match self {
            RafxMemoryUsage::Unknown => None,
            RafxMemoryUsage::GpuOnly => Some(gles20::STATIC_DRAW),
            RafxMemoryUsage::CpuOnly => Some(gles20::NONE),
            RafxMemoryUsage::CpuToGpu => Some(gles20::DYNAMIC_DRAW),
            RafxMemoryUsage::GpuToCpu => Some(gles20::STREAM_DRAW),
        }
    }
}
