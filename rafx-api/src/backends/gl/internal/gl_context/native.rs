
use raw_window_handle::HasRawWindowHandle;
use raw_gl_context::GlConfig;
use super::gles20::Gles2;
use super::gles20;
use super::gles20::types::GLenum;
use fnv::{FnvHasher, FnvHashMap};
use std::hash::{Hasher, Hash};
use super::WindowHash;
use crate::{RafxResult, RafxError};
use crate::gl::gles20::types::{GLsizeiptr, GLint};
use std::ffi::{CStr, CString};
use std::ops::Range;
use crate::gl::{ProgramId, ShaderId, BufferId, ActiveUniformInfo};
use std::cmp::max;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocationId(u32);

pub struct GetActiveUniformMaxNameLengthHint(i32);

pub struct GlContext {
    context: raw_gl_context::GlContext,
    gles2: Gles2,
    window_hash: WindowHash,
}

impl PartialEq for GlContext {
    fn eq(&self, other: &Self) -> bool {
        self.window_hash == other.window_hash
    }
}

impl GlContext {
    pub fn new(window: &dyn HasRawWindowHandle, share: Option<&GlContext>) -> Self {
        let window_hash = super::calculate_window_hash(window);

        let context = raw_gl_context::GlContext::create(window, GlConfig::default(), share.map(|x| x.context())).unwrap();
        context.make_current();
        let gles2 = Gles2::load_with(|symbol| context.get_proc_address(symbol) as *const _);
        context.make_not_current();

        GlContext {
            context,
            gles2,
            window_hash
        }
    }

    pub fn window_hash(&self) -> WindowHash {
        self.window_hash
    }

    pub fn context(&self) -> &raw_gl_context::GlContext {
        &self.context
    }

    pub fn gles2(&self) -> &Gles2 {
        &self.gles2
    }

    pub fn make_current(&self) {
        self.context.make_current();
    }

    pub fn make_not_current(&self) {
        self.context.make_not_current();
    }

    pub fn swap_buffers(&self) {
        self.context.swap_buffers();
    }

    pub fn check_for_error(&self) -> RafxResult<()> {
        unsafe {
            let result = self.gles2.GetError();
            if result != gles20::NO_ERROR {
                Err(RafxError::GlError(result))
            } else {
                Ok(())
            }
        }
    }

    pub fn gl_get_integerv(&self, pname: u32) -> i32 {
        unsafe {
            let mut value = 0;
            self.gles2.GetIntegerv(pname, &mut value);
            value
        }
    }

    pub fn gl_get_string(&self, pname: u32) -> String {
        unsafe {
            let str = self.gles2.GetString(pname);
            if str.is_null() {
                return "".to_string();
            }

            std::ffi::CStr::from_ptr(str as _).to_str().unwrap().to_string()
        }
    }

    pub fn gl_viewport(&self, x: i32, y: i32, width: i32, height: i32) -> RafxResult<()> {
        unsafe {
            self.gles2.Viewport(x, y, width, height);
            self.check_for_error()
        }
    }

    pub fn gl_clear_color(&self, r: f32, g: f32, b: f32, a: f32) -> RafxResult<()> {
        unsafe {
            self.gles2.ClearColor(r, g, b, a);
            self.check_for_error()
        }
    }

    pub fn gl_clear(&self, mask: u32) -> RafxResult<()> {
        unsafe {
            self.gles2.Clear(mask);
            self.check_for_error()
        }
    }

    pub fn gl_finish(&self) -> RafxResult<()> {
        unsafe {
            self.gles2.Finish();
            self.check_for_error()
        }
    }

    pub fn gl_create_buffer(&self) -> RafxResult<BufferId> {
        unsafe {
            let mut buffer = 0;
            self.gles2.GenBuffers(1, &mut buffer);
            self.check_for_error()?;
            Ok(BufferId(buffer))
        }
    }

    pub fn gl_destroy_buffer(&self, buffer_id: BufferId) -> RafxResult<()> {
        unsafe {
            self.gles2.DeleteBuffers(1, &buffer_id.0);
            self.check_for_error()
        }
    }

    pub fn gl_bind_buffer(&self, target: GLenum, buffer_id: BufferId) -> RafxResult<()> {
        unsafe {
            self.gles2.BindBuffer(target, buffer_id.0);
            self.check_for_error()
        }
    }

    pub fn gl_buffer_data(&self, target: GLenum, size: u64, data: *const std::ffi::c_void, usage: GLenum) -> RafxResult<()> {
        unsafe {
            self.gles2.BufferData(target, size as _, data, usage);
            self.check_for_error()
        }
    }

    pub fn gl_buffer_sub_data(&self, target: GLenum, offset: u32, size: u64, data: *const u8) -> RafxResult<()> {
        unsafe {
            self.gles2.BufferSubData(target, offset as _, size as _, data as _);
            self.check_for_error()
        }
    }

    pub fn gl_create_shader(&self, shader_type: GLenum) -> RafxResult<ShaderId> {
        unsafe {
            let id = self.gles2.CreateShader(shader_type);
            self.check_for_error()?;
            Ok(ShaderId(id))
        }
    }

    pub fn gl_destroy_shader(&self, shader_id: ShaderId) -> RafxResult<()> {
        unsafe {
            self.gles2.DeleteShader(shader_id.0);
            self.check_for_error()
        }
    }

    pub fn gl_shader_source(&self, shader_id: ShaderId, code: &CString) -> RafxResult<()> {
        unsafe {
            let len : GLint = code.as_bytes().len() as _;
            self.gles2.ShaderSource(shader_id.0, 1, &code.as_ptr(), &len);
            self.check_for_error()
        }
    }

    pub fn gl_compile_shader(&self, shader_id: ShaderId) -> RafxResult<()> {
        unsafe {
            self.gles2.CompileShader(shader_id.0);
            self.check_for_error()
        }
    }

    pub fn gl_get_shaderiv(&self, shader_id: ShaderId, pname: GLenum) -> RafxResult<i32> {
        unsafe {
            let mut value = 0;
            self.gles2.GetShaderiv(shader_id.0, pname, &mut value);
            self.check_for_error()?;
            Ok(value)
        }
    }

    pub fn gl_get_programiv(&self, program_id: ProgramId, pname: GLenum) -> RafxResult<i32> {
        unsafe {
            let mut value = 0;
            self.gles2.GetProgramiv(program_id.0, pname, &mut value);
            self.check_for_error()?;
            Ok(value)
        }
    }

    fn gl_get_shader_info_log(&self, shader_id: ShaderId, string: &mut [u8]) -> RafxResult<()> {
        unsafe {
            let mut len = string.len();
            self.gles2.GetShaderInfoLog(shader_id.0, len as _, std::ptr::null_mut(), string.as_mut_ptr() as _);
            self.check_for_error()
        }
    }

    fn gl_get_program_info_log(&self, program_id: ProgramId, string: &mut [u8]) -> RafxResult<()> {
        unsafe {
            let mut len = string.len();
            self.gles2.GetProgramInfoLog(program_id.0, len as _, std::ptr::null_mut(), string.as_mut_ptr() as _);
            self.check_for_error()
        }
    }

    pub fn get_shader_info_log(&self, shader_id: ShaderId) -> RafxResult<Option<String>> {
        let error_len = self.gl_get_shaderiv(shader_id, gles20::INFO_LOG_LENGTH)?;
        if error_len == 0 {
            return Ok(None);
        };

        let mut log = vec![0_u8; error_len as usize];
        self.gl_get_shader_info_log(shader_id, &mut log)?;
        Ok(Some(String::from_utf8(log).unwrap()))
    }

    pub fn compile_shader(&self, shader_type: GLenum, src: &CString) -> RafxResult<ShaderId> {
        let shader_id = self.gl_create_shader(shader_type)?;
        self.gl_shader_source(shader_id, &src)?;
        self.gl_compile_shader(shader_id)?;
        if self.gl_get_shaderiv(shader_id, gles20::COMPILE_STATUS)? == 0 {
            return Err(match self.get_shader_info_log(shader_id)? {
                Some(x) => format!("Error compiling shader: {}", x),
                None => "Error compiling shader, info log not available".to_string()
            })?;
        }

        if let Ok(Some(debug_info)) = self.get_shader_info_log(shader_id) {
            log::debug!("Debug info while compiling shader program: {}", debug_info);
        }

        Ok(shader_id)
    }

    pub fn gl_create_program(&self) -> RafxResult<ProgramId> {
        unsafe {
            let program_id = self.gles2.CreateProgram();
            self.check_for_error()?;
            Ok(ProgramId(program_id))
        }
    }

    pub fn gl_destroy_program(&self, program_id: ProgramId) -> RafxResult<()> {
        unsafe {
            self.gles2.DeleteProgram(program_id.0);
            self.check_for_error()
        }
    }

    pub fn gl_attach_shader(&self, program_id: ProgramId, shader_id: ShaderId) -> RafxResult<()> {
        unsafe {
            self.gles2.AttachShader(program_id.0, shader_id.0);
            self.check_for_error()
        }
    }

    pub fn gl_link_program(&self, program_id: ProgramId) -> RafxResult<()> {
        unsafe {
            self.gles2.LinkProgram(program_id.0);
            self.check_for_error()
        }
    }

    pub fn gl_validate_program(&self, program_id: ProgramId) -> RafxResult<()> {
        unsafe {
            self.gles2.ValidateProgram(program_id.0);
            self.check_for_error()
        }
    }

    fn get_program_info_log(&self, program_id: ProgramId) -> RafxResult<Option<String>> {
        let error_len = self.gl_get_programiv(program_id, gles20::INFO_LOG_LENGTH)?;
        if error_len == 0 {
            return Ok(None);
        };

        let mut log = vec![0_u8; error_len as usize];
        self.gl_get_program_info_log(program_id, &mut log)?;
        Ok(Some(String::from_utf8(log).unwrap()))
    }

    pub fn link_shader_program(&self, program_id: ProgramId) -> RafxResult<()> {
        self.gl_link_program(program_id)?;
        if self.gl_get_programiv(program_id, gles20::LINK_STATUS)? == 0 {
            return Err(match self.get_program_info_log(program_id)? {
                Some(x) => format!("Error linking shader program: {}", x),
                None => "Error linking shader program, info log not available".to_string()
            })?;
        }

        if let Ok(Some(debug_info)) = self.get_program_info_log(program_id) {
            log::debug!("Debug info while linking shader program: {}", debug_info);
        }

        Ok(())
    }

    pub fn validate_shader_program(&self, program_id: ProgramId) -> RafxResult<()> {
        self.gl_validate_program(program_id)?;
        if self.gl_get_programiv(program_id, gles20::VALIDATE_STATUS)? == 0 {
            return Err(match self.get_program_info_log(program_id)? {
                Some(x) => format!("Error validating shader program: {}", x),
                None => "Error validating shader program, info log not available".to_string()
            })?;
        }

        if let Ok(Some(debug_info)) = self.get_program_info_log(program_id) {
            log::debug!("Debug info while validating shader program: {}", debug_info);
        }

        Ok(())
    }

    pub fn gl_get_uniform_location(&self, program_id: ProgramId, name: &CStr) -> RafxResult<Option<LocationId>> {
        unsafe {
            let value = self.gles2.GetUniformLocation(program_id.0, name.as_ptr());
            self.check_for_error()?;

            if value == -1 {
                return Ok(None);
            }

            Ok(Some(LocationId(value as u32)))
        }
    }

    pub fn get_active_uniform_max_name_length_hint(&self, program_id: ProgramId) -> RafxResult<GetActiveUniformMaxNameLengthHint> {
        let max_length = self.gl_get_programiv(program_id, gles20::ACTIVE_UNIFORM_MAX_LENGTH)?;
        Ok(GetActiveUniformMaxNameLengthHint(max_length))
    }

    pub fn gl_get_active_uniform(
        &self,
        program_id: ProgramId,
        index: u32,
        max_name_length_hint: &GetActiveUniformMaxNameLengthHint
    ) -> RafxResult<ActiveUniformInfo> {
        let mut name_length = 0;
        let mut size = 0;
        let mut ty = 0;
        let mut name_buffer = vec![0_u8; max_name_length_hint.0 as usize];

        unsafe {
            self.gles2.GetActiveUniform(
                program_id.0,
                index,
                max_name_length_hint.0 as _,
                &mut name_length,
                &mut size,
                &mut ty,
                name_buffer.as_mut_ptr() as _
            );
        }
        self.check_for_error()?;

        let name = CString::new(&name_buffer[0..name_length as usize]).unwrap();

        Ok(ActiveUniformInfo {
            name,
            size: size as u32,
            ty
        })
    }

    pub fn gl_flush(&self) -> RafxResult<()> {
        unsafe {
            self.gles2.Flush();
            self.check_for_error()
        }
    }

    pub fn gl_disable(&self, value: GLenum) -> RafxResult<()> {
        unsafe {
            self.gles2.Disable(value);
            self.check_for_error()
        }
    }
}
