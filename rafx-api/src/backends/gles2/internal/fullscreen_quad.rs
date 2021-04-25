use crate::gles2::{gles2_bindings, ProgramId, RafxTextureGles2, BufferId, GlContext, NONE_PROGRAM, NONE_TEXTURE, NONE_BUFFER};
use crate::{RafxResult, RafxDeviceInfo};
use std::ffi::CString;

pub(crate) struct FullscreenQuad {
    program_id: ProgramId,
    buffer_id: BufferId,
}

impl FullscreenQuad {
    pub(crate) fn new(gl_context: &GlContext) -> RafxResult<Self> {
        let vert_shader_src = CString::new(include_str!("shaders/fullscreen_quad.vert.gles")).unwrap();
        let frag_shader_src = CString::new(include_str!("shaders/fullscreen_quad.frag.gles")).unwrap();

        let vert_shader = gl_context.compile_shader(gles2_bindings::VERTEX_SHADER, &vert_shader_src)?;
        let frag_shader = gl_context.compile_shader(gles2_bindings::FRAGMENT_SHADER, &frag_shader_src)?;

        let program_id = gl_context.gl_create_program()?;
        gl_context.gl_attach_shader(program_id, vert_shader)?;
        gl_context.gl_attach_shader(program_id, frag_shader)?;

        gl_context.gl_bind_attrib_location(program_id, 0, "pos")?;
        gl_context.gl_bind_attrib_location(program_id, 1, "uv")?;

        gl_context.link_shader_program(program_id)?;

        gl_context.gl_destroy_shader(vert_shader)?;
        gl_context.gl_destroy_shader(frag_shader)?;

        #[rustfmt::skip]
        const QUAD_VERTICES :[f32; 24] = [
            -1.0, 1.0, 0.0, 1.0,
            -1.0, -1.0, 0.0, 0.0,
            1.0, -1.0, 1.0, 0.0,
            -1.0, 1.0, 0.0, 1.0,
            1.0, -1.0, 1.0, 0.0,
            1.0, 1.0, 1.0, 1.0
        ];

        let buffer_id = gl_context.gl_create_buffer()?;
        gl_context.gl_bind_buffer(gles2_bindings::ARRAY_BUFFER, buffer_id)?;
        gl_context.gl_buffer_data(gles2_bindings::ARRAY_BUFFER, 24 * 4, QUAD_VERTICES.as_ptr() as _, gles2_bindings::STATIC_DRAW)?;

        Ok(FullscreenQuad {
            program_id,
            buffer_id,
        })
    }

    pub(crate) fn draw(&self, gl_context: &GlContext, device_info: &RafxDeviceInfo, texture: &RafxTextureGles2) -> RafxResult<()> {
        gl_context.gl_disable(gles2_bindings::DEPTH_TEST)?;
        gl_context.gl_use_program(self.program_id)?;

        gl_context.gl_bind_buffer(gles2_bindings::ARRAY_BUFFER, self.buffer_id)?;

        gl_context.gl_vertex_attrib_pointer(0, 2, gles2_bindings::FLOAT, false, 16, 0)?;
        gl_context.gl_enable_vertex_attrib_array(0)?;

        gl_context.gl_vertex_attrib_pointer(1, 2, gles2_bindings::FLOAT, false, 16, 8)?;
        gl_context.gl_enable_vertex_attrib_array(1)?;

        for i in 2..device_info.max_vertex_attribute_count {
            gl_context.gl_disable_vertex_attrib_array(i)?;
        }

        gl_context.gl_bind_texture(gles2_bindings::TEXTURE_2D, texture.gl_raw_image().gl_texture_id().unwrap())?;
        gl_context.gl_tex_parameteri(gles2_bindings::TEXTURE_2D, gles2_bindings::TEXTURE_MIN_FILTER, gles2_bindings::LINEAR as _)?;
        gl_context.gl_tex_parameteri(gles2_bindings::TEXTURE_2D, gles2_bindings::TEXTURE_MAG_FILTER, gles2_bindings::LINEAR as _)?;
        gl_context.gl_tex_parameteri(gles2_bindings::TEXTURE_2D, gles2_bindings::TEXTURE_WRAP_S, gles2_bindings::CLAMP_TO_EDGE as _)?;
        gl_context.gl_tex_parameteri(gles2_bindings::TEXTURE_2D, gles2_bindings::TEXTURE_WRAP_T, gles2_bindings::CLAMP_TO_EDGE as _)?;
        gl_context.gl_draw_arrays(gles2_bindings::TRIANGLES, 0, 6)?;

        gl_context.gl_bind_buffer(gles2_bindings::ARRAY_BUFFER, NONE_BUFFER)?;
        gl_context.gl_bind_texture(gles2_bindings::TEXTURE_2D, NONE_TEXTURE)?;
        gl_context.gl_use_program(NONE_PROGRAM)?;

        Ok(())
    }

    pub(crate) fn destroy(&self, gl_context: &GlContext) -> RafxResult<()> {
        gl_context.gl_destroy_program(self.program_id)?;
        gl_context.gl_destroy_buffer(self.buffer_id)?;
        Ok(())
    }
}