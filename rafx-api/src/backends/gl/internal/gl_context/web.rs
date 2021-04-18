use raw_window_handle::HasRawWindowHandle;
use web_sys::{WebGlRenderingContext, WebGlBuffer, WebGlShader};
use crate::{RafxResult, RafxError};
use crate::gl::{gles20, ProgramId, ShaderId, BufferId, WindowHash, ActiveUniformInfo, NONE_BUFFER};
use crate::gl::gles20::types::*;
use std::ffi::{CString, CStr};
use fnv::FnvHashMap;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use wasm_bindgen::JsValue;

static NEXT_GL_BUFFER_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
static NEXT_GL_SHADER_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

fn convert_js_to_i32(value: &JsValue) -> Option<i32> {
    if let Some(value) = value.as_f64() {
        Some(value as i32)
    } else if let Some(value) = value.as_bool() {
        if value {
            Some(1)
        } else {
            Some(0)
        }
    } else {
        None
    }
}

pub struct GlContext {
    context: WebGlRenderingContext,
    window_hash: WindowHash,
    buffers: Mutex<FnvHashMap<BufferId, WebGlBuffer>>,
    shaders: Mutex<FnvHashMap<ShaderId, WebGlShader>>,
}

impl PartialEq for GlContext {
    fn eq(&self, other: &Self) -> bool {
        self.window_hash == other.window_hash
    }
}

impl GlContext {
    pub fn new(window: &dyn HasRawWindowHandle, share: Option<&GlContext>) -> Self {
        if share.is_some() {
            panic!("rafx-api web does not support multiple GL contexts");
        }

        use wasm_bindgen::JsCast;
        let handle = if let raw_window_handle::RawWindowHandle::Web(handle) = window.raw_window_handle() {
            Some(handle.id)
        } else {
            None
        }.unwrap();

        let canvas: web_sys::HtmlCanvasElement = web_sys::window()
            .and_then(|win| win.document())
            .expect("Cannot get document")
            .query_selector(&format!("canvas[data-raw-handle=\"{}\"]", handle))
            .expect("Cannot query for canvas")
            .expect("Canvas is not found")
            .dyn_into()
            .expect("Failed to downcast to canvas type");

        let context = canvas
            .get_context("webgl")
            .unwrap()
            .unwrap()
            .dyn_into::<WebGlRenderingContext>()
            .unwrap();

        let window_hash = super::calculate_window_hash(window);

        GlContext {
            context,
            window_hash,
            buffers: Default::default(),
            shaders: Default::default()
        }
    }

    pub fn window_hash(&self) -> WindowHash {
        self.window_hash
    }

    pub fn context(&self) -> &WebGlRenderingContext {
        &self.context
    }

    pub fn make_current(&self) {
        // Web does not support multiple threads so this is irrelevant
    }

    pub fn make_not_current(&self) {
        // Web does not support multiple threads so this is irrelevant
    }

    pub fn swap_buffers(&self) {
        // Web swaps the buffers for us so this is irrelevant
    }

    pub fn check_for_error(&self) -> RafxResult<()> {
        let result = self.context.get_error();
        if result != gles20::NO_ERROR {
            Err(RafxError::GlError(result))
        } else {
            Ok(())
        }
    }

    pub fn gl_get_integerv(&self, pname: u32) -> i32 {
        convert_js_to_i32(&self.context.get_parameter(pname).unwrap()).unwrap()
    }

    pub fn gl_get_string(&self, pname: u32) -> String {
        self.context.get_parameter(pname).unwrap().as_string().unwrap()
    }

    pub fn gl_viewport(&self, x: i32, y: i32, width: i32, height: i32) {
        self.context.viewport(x, y, width, height)
    }

    pub fn gl_clear_color(&self, r: f32, g: f32, b: f32, a: f32) {
        self.context.clear_color(r, g, b, a);
    }

    pub fn gl_clear(&self, mask: u32) {
        self.context.clear(mask);
    }

    pub fn gl_finish(&self) -> RafxResult<()> {
        self.context.finish();
        self.check_for_error()
    }

    pub fn gl_create_buffer(&self) -> RafxResult<BufferId> {
        let buffer = self.context.create_buffer().unwrap();
        self.check_for_error()?;
        let buffer_id = BufferId(NEXT_GL_BUFFER_ID.fetch_add(1, Ordering::Relaxed));
        let old = self.buffers.lock().unwrap().insert(buffer_id, buffer);
        assert!(old.is_none());
        Ok(buffer_id)
    }

    pub fn gl_destroy_buffer(&self, buffer_id: BufferId) -> RafxResult<()> {
        let buffer = self.buffers.lock().unwrap().remove(&buffer_id).unwrap();
        self.context.delete_buffer(Some(&buffer));
        self.check_for_error()
    }

    pub fn gl_bind_buffer(&self, target: GLenum, buffer_id: BufferId) -> RafxResult<()> {
        if buffer_id == NONE_BUFFER {
            self.context.bind_buffer(target, None);
        } else {
            let buffers = self.buffers.lock().unwrap();
            let buffer = buffers.get(&buffer_id).unwrap();
            self.context.bind_buffer(target, Some(buffer));
        }

        self.check_for_error()
    }

    pub fn gl_buffer_data(&self, target: GLenum, size: u64, data: *const std::ffi::c_void, usage: GLenum) -> RafxResult<()> {
        unsafe {
            let slice = std::slice::from_raw_parts(data as *const u8, size as usize);
            self.context.buffer_data_with_u8_array(target, slice, usage);
        }
        self.check_for_error()
    }

    pub fn gl_buffer_sub_data(&self, target: GLenum, offset: u32, size: u64, data: *const u8) -> RafxResult<()> {
        unsafe {
            let slice = std::slice::from_raw_parts(data as *const u8, size as usize);
            self.context.buffer_sub_data_with_i32_and_u8_array(target, offset as i32, slice);
        }
        self.check_for_error()
    }

    pub fn gl_create_shader(&self, shader_type: GLenum) -> RafxResult<ShaderId> {
        let shader = self.context.create_shader(shader_type).unwrap();
        self.check_for_error()?;
        let shader_id = ShaderId(NEXT_GL_SHADER_ID.fetch_add(1, Ordering::Relaxed));
        let old = self.shaders.lock().unwrap().insert(shader_id, shader);
        assert!(old.is_none());
        Ok(shader_id)
    }

    pub fn gl_destroy_shader(&self, shader_id: ShaderId) -> RafxResult<()> {
        let shader = self.shaders.lock().unwrap().remove(&shader_id).unwrap();
        self.context.delete_shader(Some(&shader));
        self.check_for_error()
    }

    pub fn gl_shader_source(&self, shader_id: ShaderId, code: &CString) -> RafxResult<()> {
        let shaders = self.shaders.lock().unwrap();
        self.context.shader_source(shaders.get(&shader_id).unwrap(), code.to_str().unwrap());
        self.check_for_error()
    }

    pub fn gl_compile_shader(&self, shader_id: ShaderId) -> RafxResult<()> {
        let shaders = self.shaders.lock().unwrap();
        self.context.compile_shader(shaders.get(&shader_id).unwrap());
        self.check_for_error()
    }

    pub fn gl_get_shaderiv(&self, shader_id: ShaderId, pname: GLenum) -> RafxResult<i32> {
        let shaders = self.shaders.lock().unwrap();
        let value = self.context.get_shader_parameter(shaders.get(&shader_id).unwrap(), pname);
        self.check_for_error()?;
        Ok(convert_js_to_i32(&value).ok_or_else(|| format!("Parameter {} in convert_js_to_i32 is a {:?} which is neither a number or boolean", pname, value))?)
    }

    pub fn gl_get_programiv(&self, program_id: ProgramId, pname: GLenum) -> RafxResult<i32> {
        log::trace!("gl_get_programiv unimplemented");
        // unsafe {
        //     let mut value = 0;
        //     self.gles2.GetProgramiv(program_id.0, pname, &mut value);
        //     self.check_for_error()?;
        //     Ok(value)
        // }
        unimplemented!();
    }

    // pub fn gl_get_shader_info_log(&self, shader_id: ShaderId, string: &mut [u8]) -> RafxResult<()> {
    //     let shaders = self.shaders.lock().unwrap();
    //     let value = self.context.get_shader_info_log(shaders.get(&shader_id).unwrap()).ok_or("Shader log unavailable")?;
    //
    //     let log_cstring = CString::new(value).unwrap();
    //     let log_bytes = log_cstring.to_bytes();
    //     for i in 0..string.len().min(log_bytes.len()) {
    //         string[i] = log_bytes[i];
    //     }
    //     string[string.len() - 1] = 0;
    //
    //     Ok(())
    // }

    // pub fn gl_get_program_info_log(&self, program_id: ProgramId, string: &mut [u8]) -> RafxResult<()> {
    //     log::trace!("gl_get_program_info_log unimplemented");
    //     // unsafe {
    //     //     let mut len = string.len();
    //     //     self.gles2.GetProgramInfoLog(program_id.0, len as _, std::ptr::null_mut(), string.as_mut_ptr() as _);
    //     //     self.check_for_error()
    //     // }
    //     unimplemented!();
    // }

    fn get_shader_info_log(&self, shader_id: ShaderId) -> RafxResult<Option<String>> {
        let shaders = self.shaders.lock().unwrap();
        let value = self.context.get_shader_info_log(shaders.get(&shader_id).unwrap()).ok_or("Shader log unavailable")?;

        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }

    pub fn compile_shader(&self, shader_type: GLenum, src: &CString) -> RafxResult<ShaderId> {
        log::trace!("compiling shader");
        let shader_id = self.gl_create_shader(shader_type)?;
        self.gl_shader_source(shader_id, &src)?;
        self.gl_compile_shader(shader_id)?;
        log::trace!("compiling shader AAAAA");
        if self.gl_get_shaderiv(shader_id, gles20::COMPILE_STATUS)? == 0 {
            return Err(match self.get_shader_info_log(shader_id)? {
                Some(x) => format!("Error compiling shader: {}", x),
                None => "Error compiling shader, info log not available".to_string()
            })?;
        }

        log::trace!("compiling shader BBBBB");
        if let Ok(Some(debug_info)) = self.get_shader_info_log(shader_id) {
            log::debug!("Debug info while compiling shader program: {}", debug_info);
        }

        log::trace!("compiled shader");

        Ok(shader_id)
    }

    pub fn gl_create_program(&self) -> RafxResult<ProgramId> {
        log::trace!("gl_create_program unimplemented");
        // unsafe {
        //     let program_id = self.gles2.CreateProgram();
        //     self.check_for_error()?;
        //     Ok(ProgramId(program_id))
        // }
        unimplemented!();
    }

    pub fn gl_attach_shader(&self, program_id: ProgramId, shader_id: ShaderId) -> RafxResult<()> {
        log::trace!("gl_attach_shader unimplemented");
        // unsafe {
        //     self.gles2.AttachShader(program_id.0, shader_id.0);
        //     self.check_for_error()
        // }
        unimplemented!();
    }

    pub fn gl_link_program(&self, program_id: ProgramId) -> RafxResult<()> {
        log::trace!("gl_link_program unimplemented");
        // unsafe {
        //     self.gles2.LinkProgram(program_id.0);
        //     self.check_for_error()
        // }
        unimplemented!();
    }

    pub fn gl_validate_program(&self, program_id: ProgramId) -> RafxResult<()> {
        log::trace!("gl_validate_program unimplemented");
        // unsafe {
        //     self.gles2.ValidateProgram(program_id.0);
        //     self.check_for_error()
        // }
        unimplemented!();
    }

    fn get_program_info_log(&self, program_id: ProgramId) -> RafxResult<Option<String>> {
        log::trace!("get_program_info_log unimplemented");
        // let error_len = self.gl_get_programiv(program_id, gles20::INFO_LOG_LENGTH)?;
        // if error_len == 0 {
        //     return Ok(None);
        // };
        //
        // let mut log = vec![0_u8; error_len as usize];
        // self.gl_get_program_info_log(program_id, &mut log)?;
        // Ok(Some(String::from_utf8(log).unwrap()))
        unimplemented!();
    }

    pub fn link_shader_program(&self, program_id: ProgramId) -> RafxResult<()> {
        log::trace!("link_shader_program unimplemented");
        // self.gl_link_program(program_id)?;
        // if self.gl_get_programiv(program_id, gles20::LINK_STATUS)? == 0 {
        //     return Err(match self.get_program_info_log(program_id)? {
        //         Some(x) => format!("Error linking shader program: {}", x),
        //         None => "Error linking shader program, info log not available".to_string()
        //     })?;
        // }
        //
        // if let Ok(Some(debug_info)) = self.get_program_info_log(program_id) {
        //     log::debug!("Debug info while linking shader program: {}", debug_info);
        // }
        //
        // Ok(())
        unimplemented!();
    }

    pub fn validate_shader_program(&self, program_id: ProgramId) -> RafxResult<()> {
        log::trace!("validate_shader_program unimplemented");
        // self.gl_validate_program(program_id)?;
        // if self.gl_get_programiv(program_id, gles20::VALIDATE_STATUS)? == 0 {
        //     return Err(match self.get_program_info_log(program_id)? {
        //         Some(x) => format!("Error validating shader program: {}", x),
        //         None => "Error validating shader program, info log not available".to_string()
        //     })?;
        // }
        //
        // if let Ok(Some(debug_info)) = self.get_program_info_log(program_id) {
        //     log::debug!("Debug info while validating shader program: {}", debug_info);
        // }
        //
        // Ok(())
        unimplemented!();
    }

    pub fn gl_get_uniform_location(&self, program_id: ProgramId, name: &CStr) -> RafxResult<Option<u32>> {
        log::trace!("gl_get_uniform_location unimplemented");
        // unsafe {
        //     let value = self.gles2.GetUniformLocation(program_id.0, name.as_ptr());
        //     self.check_for_error()?;
        //
        //     if value == -1 {
        //         return Ok(None);
        //     }
        //
        //     Ok(Some(value as u32))
        // }
        unimplemented!();
    }


    pub fn gl_get_active_uniform(
        &self,
        program_id: ProgramId,
        index: u32,
        max_uniform_name_length: usize
    ) -> RafxResult<ActiveUniformInfo> {
        log::trace!("gl_get_active_uniform unimplemented");
        // let mut name_length = 0;
        // let mut size = 0;
        // let mut ty = 0;
        // let mut name_buffer = vec![0_u8; max_uniform_name_length];
        //
        // unsafe {
        //     self.gles2.GetActiveUniform(
        //         program_id.0,
        //         index,
        //         max_uniform_name_length as _,
        //         &mut name_length,
        //         &mut size,
        //         &mut ty,
        //         name_buffer.as_mut_ptr() as _
        //     );
        // }
        // self.check_for_error()?;
        //
        // name_buffer.resize(name_length as usize, 0);
        //
        // Ok(ActiveUniformInfo {
        //     name_buffer,
        //     size: size as u32,
        //     ty
        // })
        unimplemented!();
    }

    pub fn gl_flush(&self) -> RafxResult<()> {
        log::trace!("gl_flush unimplemented");
        // unsafe {
        //     self.gles2.Flush();
        //     self.check_for_error()
        // }
        unimplemented!();
    }

    pub fn gl_disable(&self, value: GLenum) -> RafxResult<()> {
        log::trace!("gl_disable unimplemented");
        // unsafe {
        //     self.gles2.Disable(value);
        //     self.check_for_error()
        // }
        unimplemented!();
    }
}