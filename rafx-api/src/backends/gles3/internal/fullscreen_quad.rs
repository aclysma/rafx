use crate::gles3::{
    gles3_bindings, BufferId, GlContext, ProgramId, RafxTextureGles3, NONE_BUFFER, NONE_PROGRAM,
    NONE_TEXTURE,
};
use crate::RafxResult;
use std::ffi::CString;

pub(crate) struct FullscreenQuad {
    program_id: ProgramId,
    buffer_id: BufferId,
    flip_y_buffer_id: BufferId,
}

impl FullscreenQuad {
    pub(crate) fn new(gl_context: &GlContext) -> RafxResult<Self> {
        let vert_shader_src =
            CString::new(include_str!("shaders/fullscreen_quad.vert.gles")).unwrap();
        let frag_shader_src =
            CString::new(include_str!("shaders/fullscreen_quad.frag.gles")).unwrap();

        let vert_shader =
            gl_context.compile_shader(gles3_bindings::VERTEX_SHADER, &vert_shader_src)?;
        let frag_shader =
            gl_context.compile_shader(gles3_bindings::FRAGMENT_SHADER, &frag_shader_src)?;

        let program_id = gl_context.gl_create_program()?;
        gl_context.gl_attach_shader(program_id, vert_shader)?;
        gl_context.gl_attach_shader(program_id, frag_shader)?;

        gl_context.gl_bind_attrib_location(program_id, 0, "pos")?;
        gl_context.gl_bind_attrib_location(program_id, 1, "uv")?;

        gl_context.link_shader_program(program_id)?;

        gl_context.gl_destroy_shader(vert_shader)?;
        gl_context.gl_destroy_shader(frag_shader)?;

        #[rustfmt::skip]
        const FLIP_Y_QUAD_VERTICES :[f32; 24] = [
            -1.0, 1.0, 0.0, 0.0,
            -1.0, -1.0, 0.0, 1.0,
            1.0, -1.0, 1.0, 1.0,
            -1.0, 1.0, 0.0, 0.0,
            1.0, -1.0, 1.0, 1.0,
            1.0, 1.0, 1.0, 0.0
        ];

        let flip_y_buffer_id = gl_context.gl_create_buffer()?;
        gl_context.gl_bind_buffer(gles3_bindings::ARRAY_BUFFER, flip_y_buffer_id)?;
        gl_context.gl_buffer_data(
            gles3_bindings::ARRAY_BUFFER,
            24 * 4,
            FLIP_Y_QUAD_VERTICES.as_ptr() as _,
            gles3_bindings::STATIC_DRAW,
        )?;

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
        gl_context.gl_bind_buffer(gles3_bindings::ARRAY_BUFFER, buffer_id)?;
        gl_context.gl_buffer_data(
            gles3_bindings::ARRAY_BUFFER,
            24 * 4,
            QUAD_VERTICES.as_ptr() as _,
            gles3_bindings::STATIC_DRAW,
        )?;

        Ok(FullscreenQuad {
            program_id,
            buffer_id,
            flip_y_buffer_id,
        })
    }

    pub(crate) fn draw(
        &self,
        gl_context: &GlContext,
        texture: &RafxTextureGles3,
        flip_y: bool,
    ) -> RafxResult<()> {
        gl_context.gl_disable(gles3_bindings::BLEND)?;
        gl_context.gl_disable(gles3_bindings::DEPTH_TEST)?;
        gl_context.gl_use_program(self.program_id)?;

        let buffer_id = if flip_y {
            self.flip_y_buffer_id
        } else {
            self.buffer_id
        };

        gl_context.gl_bind_buffer(gles3_bindings::ARRAY_BUFFER, buffer_id)?;

        gl_context.gl_vertex_attrib_pointer(0, 2, gles3_bindings::FLOAT, false, 16, 0)?;
        gl_context.gl_enable_vertex_attrib_array(0)?;

        gl_context.gl_vertex_attrib_pointer(1, 2, gles3_bindings::FLOAT, false, 16, 8)?;
        gl_context.gl_enable_vertex_attrib_array(1)?;

        gl_context.gl_active_texture(0)?;
        gl_context.gl_bind_texture(
            gles3_bindings::TEXTURE_2D,
            texture.gl_raw_image().gl_texture_id().unwrap(),
        )?;
        gl_context.gl_tex_parameteri(
            gles3_bindings::TEXTURE_2D,
            gles3_bindings::TEXTURE_MIN_FILTER,
            gles3_bindings::LINEAR as _,
        )?;
        gl_context.gl_tex_parameteri(
            gles3_bindings::TEXTURE_2D,
            gles3_bindings::TEXTURE_MAG_FILTER,
            gles3_bindings::LINEAR as _,
        )?;
        gl_context.gl_tex_parameteri(
            gles3_bindings::TEXTURE_2D,
            gles3_bindings::TEXTURE_WRAP_S,
            gles3_bindings::CLAMP_TO_EDGE as _,
        )?;
        gl_context.gl_tex_parameteri(
            gles3_bindings::TEXTURE_2D,
            gles3_bindings::TEXTURE_WRAP_T,
            gles3_bindings::CLAMP_TO_EDGE as _,
        )?;
        gl_context.gl_draw_arrays(gles3_bindings::TRIANGLES, 0, 6)?;

        gl_context.gl_bind_buffer(gles3_bindings::ARRAY_BUFFER, NONE_BUFFER)?;
        gl_context.gl_bind_texture(gles3_bindings::TEXTURE_2D, NONE_TEXTURE)?;
        gl_context.gl_use_program(NONE_PROGRAM)?;

        gl_context.gl_disable_vertex_attrib_array(0)?;
        gl_context.gl_disable_vertex_attrib_array(1)?;

        Ok(())
    }

    pub(crate) fn destroy(
        &self,
        gl_context: &GlContext,
    ) -> RafxResult<()> {
        gl_context.gl_destroy_program(self.program_id)?;
        gl_context.gl_destroy_buffer(self.buffer_id)?;
        Ok(())
    }
}
