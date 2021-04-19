use raw_window_handle::HasRawWindowHandle;
use web_sys::{WebGlRenderingContext, WebGlBuffer, WebGlShader, WebGlProgram, WebGlUniformLocation};
use crate::{RafxResult, RafxError};
use crate::gl::{gles20, ProgramId, ShaderId, BufferId, WindowHash, ActiveUniformInfo, NONE_BUFFER};
use crate::gl::gles20::types::*;
use std::ffi::{CString, CStr};
use fnv::FnvHashMap;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use wasm_bindgen::JsValue;

pub struct GetActiveUniformMaxNameLengthHint;

static NEXT_GL_BUFFER_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
static NEXT_GL_SHADER_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
static NEXT_GL_PROGRAM_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocationId(WebGlUniformLocation);

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
    programs: Mutex<FnvHashMap<ProgramId, WebGlProgram>>,
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
            shaders: Default::default(),
            programs: Default::default(),
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
        // This pname is undefined in webgl
        if pname == gles20::ACTIVE_UNIFORM_MAX_LENGTH {
            // Return a
            return Ok(256);
        }

        let programs = self.programs.lock().unwrap();
        let value = self.context.get_program_parameter(programs.get(&program_id).unwrap(), pname);
        self.check_for_error()?;
        Ok(convert_js_to_i32(&value).ok_or_else(|| format!("Parameter {} in convert_js_to_i32 is a {:?} which is neither a number or boolean", pname, value))?)
    }

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
        let program = self.context.create_program().unwrap();
        self.check_for_error()?;
        let program_id = ProgramId(NEXT_GL_SHADER_ID.fetch_add(1, Ordering::Relaxed));
        let old = self.programs.lock().unwrap().insert(program_id, program);
        assert!(old.is_none());
        Ok(program_id)
    }

    pub fn gl_destroy_program(&self, program_id: ProgramId) -> RafxResult<()> {
        let program = self.programs.lock().unwrap().remove(&program_id).unwrap();
        self.context.delete_program(Some(&program));
        self.check_for_error()
    }

    pub fn gl_attach_shader(&self, program_id: ProgramId, shader_id: ShaderId) -> RafxResult<()> {
        let programs = self.programs.lock().unwrap();
        let shaders = self.shaders.lock().unwrap();
        self.context.attach_shader(programs.get(&program_id).unwrap(), shaders.get(&shader_id).unwrap());
        self.check_for_error()
    }

    pub fn gl_link_program(&self, program_id: ProgramId) -> RafxResult<()> {
        let programs = self.programs.lock().unwrap();
        self.context.link_program(programs.get(&program_id).unwrap());
        self.check_for_error()
    }

    pub fn gl_validate_program(&self, program_id: ProgramId) -> RafxResult<()> {
        let programs = self.programs.lock().unwrap();
        self.context.validate_program(programs.get(&program_id).unwrap());
        self.check_for_error()
    }

    fn get_program_info_log(&self, program_id: ProgramId) -> RafxResult<Option<String>> {
        let programs = self.programs.lock().unwrap();
        let value = self.context.get_program_info_log(programs.get(&program_id).unwrap()).ok_or("Program log unavailable")?;

        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(value))
        }
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
        let programs = self.programs.lock().unwrap();
        let location = self.context.get_uniform_location(programs.get(&program_id).unwrap(), &*name.to_string_lossy());
        self.check_for_error()?;
        Ok(location.map(|x| LocationId(x)))
    }

    pub fn get_active_uniform_max_name_length_hint(&self, _program_id: ProgramId) -> RafxResult<GetActiveUniformMaxNameLengthHint> {
        Ok(GetActiveUniformMaxNameLengthHint)
    }

    pub fn gl_get_active_uniform(
        &self,
        program_id: ProgramId,
        index: u32,
        _max_name_length_hint: &GetActiveUniformMaxNameLengthHint
    ) -> RafxResult<ActiveUniformInfo> {
        let programs = self.programs.lock().unwrap();
        let info = self.context.get_active_uniform(programs.get(&program_id).unwrap(), index).ok_or_else(|| format!("Did not find uniform {} in gl_get_active_uniform", index))?;

        Ok(ActiveUniformInfo {
            name: CString::new(info.name()).unwrap(),
            size: info.size() as u32,
            ty: info.type_()
        })
    }

    pub fn gl_flush(&self) -> RafxResult<()> {
        self.context.flush();
        self.check_for_error()
    }

    pub fn gl_disable(&self, value: GLenum) -> RafxResult<()> {
        self.context.disable(value);
        self.check_for_error()
    }
}