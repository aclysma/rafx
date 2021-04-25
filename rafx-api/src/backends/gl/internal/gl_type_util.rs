use crate::gl::gles2_bindings::types::GLenum;
use crate::gl::{gles2_bindings, GlContext, LocationId};
use crate::RafxResult;

pub fn byte_size_of_type(gl_type: GLenum) -> u32 {
    match gl_type {
        gles2_bindings::INT | gles2_bindings::BOOL | gles2_bindings::FLOAT => 4,
        gles2_bindings::INT_VEC2 | gles2_bindings::BOOL_VEC2 | gles2_bindings::FLOAT_VEC2 => 8,
        gles2_bindings::INT_VEC3
        | gles2_bindings::BOOL_VEC3
        | gles2_bindings::FLOAT_VEC3
        | gles2_bindings::INT_VEC4
        | gles2_bindings::BOOL_VEC4
        | gles2_bindings::FLOAT_VEC4 => 16,
        gles2_bindings::FLOAT_MAT2 => 32,
        gles2_bindings::FLOAT_MAT3 => 48,
        gles2_bindings::FLOAT_MAT4 => 64,
        _ => unimplemented!("Unknown GL type in byte_size_of_type"),
    }
}

pub fn set_uniform<T: Copy>(
    gl_context: &GlContext,
    location: &LocationId,
    data: &T,
    gl_type: GLenum,
    count: u32,
) -> RafxResult<()> {
    match gl_type {
        gles2_bindings::INT | gles2_bindings::BOOL | gles2_bindings::SAMPLER_2D | gles2_bindings::SAMPLER_CUBE => {
            gl_context.gl_uniform_1iv(location, data, count)
        }
        gles2_bindings::FLOAT => gl_context.gl_uniform_1fv(location, data, count),
        gles2_bindings::INT_VEC2 | gles2_bindings::BOOL_VEC2 => gl_context.gl_uniform_2iv(location, data, count),
        gles2_bindings::FLOAT_VEC2 => gl_context.gl_uniform_2fv(location, data, count),
        gles2_bindings::INT_VEC3 | gles2_bindings::BOOL_VEC3 => gl_context.gl_uniform_3iv(location, data, count),
        gles2_bindings::FLOAT_VEC3 => gl_context.gl_uniform_3fv(location, data, count),
        gles2_bindings::INT_VEC4 | gles2_bindings::BOOL_VEC4 => gl_context.gl_uniform_4iv(location, data, count),
        gles2_bindings::FLOAT_VEC4 => gl_context.gl_uniform_4fv(location, data, count),
        gles2_bindings::FLOAT_MAT2 => gl_context.gl_uniform_matrix_2fv(location, data, count),
        gles2_bindings::FLOAT_MAT3 => gl_context.gl_uniform_matrix_3fv(location, data, count),
        gles2_bindings::FLOAT_MAT4 => gl_context.gl_uniform_matrix_4fv(location, data, count),
        _ => unimplemented!("Unknown GL type in set_uniform"),
    }
}
