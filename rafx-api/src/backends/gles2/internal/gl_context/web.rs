use crate::gles2::gles2_bindings::types::*;
use crate::gles2::{gles2_bindings, ActiveUniformInfo, BufferId, ProgramId, RenderbufferId, ShaderId, TextureId, WindowHash, NONE_BUFFER, NONE_PROGRAM, NONE_RENDERBUFFER, NONE_TEXTURE, FramebufferId, NONE_FRAMEBUFFER};
use crate::{RafxError, RafxResult};
use fnv::FnvHashMap;
use raw_window_handle::HasRawWindowHandle;
use std::ffi::{CStr, CString};
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use wasm_bindgen::JsValue;
use web_sys::{WebGlBuffer, WebGlProgram, WebGlRenderbuffer, WebGlRenderingContext, WebGlShader, WebGlTexture, WebGlUniformLocation, WebGlFramebuffer};

pub struct GetActiveUniformMaxNameLengthHint;

static NEXT_GL_TEXTURE_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
static NEXT_GL_BUFFER_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
static NEXT_GL_SHADER_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
static NEXT_GL_FRAMEBUFFER_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
//static NEXT_GL_PROGRAM_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
//static NEXT_GL_RENDERBUFFER_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

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
    textures: Mutex<FnvHashMap<TextureId, WebGlTexture>>,
    buffers: Mutex<FnvHashMap<BufferId, WebGlBuffer>>,
    shaders: Mutex<FnvHashMap<ShaderId, WebGlShader>>,
    programs: Mutex<FnvHashMap<ProgramId, WebGlProgram>>,
    renderbuffers: Mutex<FnvHashMap<RenderbufferId, WebGlRenderbuffer>>,
    framebuffers: Mutex<FnvHashMap<FramebufferId, WebGlFramebuffer>>,
}

impl PartialEq for GlContext {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.window_hash == other.window_hash
    }
}

impl GlContext {
    pub fn new(
        window: &dyn HasRawWindowHandle,
        share: Option<&GlContext>,
    ) -> RafxResult<Self> {
        if share.is_some() {
            panic!("rafx-api web does not support multiple GL contexts");
        }

        use wasm_bindgen::JsCast;
        let handle =
            if let raw_window_handle::RawWindowHandle::Web(handle) = window.raw_window_handle() {
                Some(handle.id)
            } else {
                None
            }
            .unwrap();

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

        Ok(GlContext {
            context,
            window_hash,
            textures: Default::default(),
            buffers: Default::default(),
            shaders: Default::default(),
            programs: Default::default(),
            renderbuffers: Default::default(),
            framebuffers: Default::default()
        })
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
        if result != gles2_bindings::NO_ERROR {
            Err(RafxError::GlError(result))
        } else {
            Ok(())
        }
    }

    pub fn gl_get_integerv(
        &self,
        pname: u32,
    ) -> i32 {
        convert_js_to_i32(&self.context.get_parameter(pname).unwrap()).unwrap()
    }

    pub fn gl_get_string(
        &self,
        pname: u32,
    ) -> String {
        self.context
            .get_parameter(pname)
            .unwrap()
            .as_string()
            .unwrap()
    }

    pub fn gl_viewport(
        &self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> RafxResult<()> {
        self.context.viewport(x, y, width, height);
        self.check_for_error()
    }

    pub fn gl_scissor(
        &self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> RafxResult<()> {
        self.context.scissor(x, y, width, height);
        self.check_for_error()
    }

    pub fn gl_depth_rangef(
        &self,
        n: f32,
        f: f32,
    ) -> RafxResult<()> {
        self.context.depth_range(n, f);
        self.check_for_error()
    }

    pub fn gl_clear_color(
        &self,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    ) -> RafxResult<()> {
        self.context.clear_color(r, g, b, a);
        self.check_for_error()
    }

    pub fn gl_clear_depthf(
        &self,
        d: f32,
    ) -> RafxResult<()> {
        self.context.clear_depth(d);
        self.check_for_error()
    }

    pub fn gl_clear_stencil(
        &self,
        s: i32,
    ) -> RafxResult<()> {
        self.context.clear_stencil(s);
        self.check_for_error()
    }

    pub fn gl_clear(
        &self,
        mask: u32,
    ) -> RafxResult<()> {
        self.context.clear(mask);
        self.check_for_error()
    }

    pub fn gl_finish(&self) -> RafxResult<()> {
        self.context.finish();
        self.check_for_error()
    }

    pub fn gl_create_framebuffer(&self) -> RafxResult<FramebufferId> {
        let framebuffer = self.context.create_framebuffer().unwrap();
        self.check_for_error()?;
        let framebuffer_id = FramebufferId(NEXT_GL_FRAMEBUFFER_ID.fetch_add(1, Ordering::Relaxed));
        let old = self.framebuffers.lock().unwrap().insert(framebuffer_id, framebuffer);
        assert!(old.is_none());
        Ok(framebuffer_id)
    }

    pub fn gl_destroy_framebuffer(
        &self,
        framebuffer_id: FramebufferId,
    ) -> RafxResult<()> {
        let framebuffer = self.framebuffers.lock().unwrap().remove(&framebuffer_id).unwrap();
        self.context.delete_framebuffer(Some(&framebuffer));
        self.check_for_error()
    }

    pub fn gl_create_texture(&self) -> RafxResult<TextureId> {
        let texture = self.context.create_texture().unwrap();
        self.check_for_error()?;
        let texture_id = TextureId(NEXT_GL_TEXTURE_ID.fetch_add(1, Ordering::Relaxed));
        let old = self.textures.lock().unwrap().insert(texture_id, texture);
        assert!(old.is_none());
        Ok(texture_id)
    }

    pub fn gl_destroy_texture(
        &self,
        texture_id: TextureId,
    ) -> RafxResult<()> {
        let texture = self.textures.lock().unwrap().remove(&texture_id).unwrap();
        self.context.delete_texture(Some(&texture));
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

    pub fn gl_destroy_buffer(
        &self,
        buffer_id: BufferId,
    ) -> RafxResult<()> {
        let buffer = self.buffers.lock().unwrap().remove(&buffer_id).unwrap();
        self.context.delete_buffer(Some(&buffer));
        self.check_for_error()
    }

    pub fn gl_bind_buffer(
        &self,
        target: GLenum,
        buffer_id: BufferId,
    ) -> RafxResult<()> {
        if buffer_id == NONE_BUFFER {
            self.context.bind_buffer(target, None);
        } else {
            let buffers = self.buffers.lock().unwrap();
            let buffer = buffers.get(&buffer_id).unwrap();
            self.context.bind_buffer(target, Some(buffer));
        }

        self.check_for_error()
    }

    pub fn gl_bind_framebuffer(
        &self,
        target: GLenum,
        framebuffer_id: FramebufferId,
    ) -> RafxResult<()> {
        if framebuffer_id == NONE_FRAMEBUFFER {
            self.context.bind_framebuffer(target, None);
        } else {
            let framebuffers = self.framebuffers.lock().unwrap();
            let framebuffer = framebuffers.get(&framebuffer_id).unwrap();
            self.context.bind_framebuffer(target, Some(framebuffer));
        }

        self.check_for_error()
    }

    pub fn gl_vertex_attrib_pointer(
        &self,
        index: u32,
        size: i32,
        type_: GLenum,
        normalized: bool,
        stride: u32,
        byte_offset: u32,
    ) -> RafxResult<()> {
        self.context.vertex_attrib_pointer_with_i32(
            index,
            size,
            type_,
            normalized,
            stride as _,
            byte_offset as _,
        );
        self.check_for_error()
    }

    pub fn gl_enable_vertex_attrib_array(
        &self,
        index: u32,
    ) -> RafxResult<()> {
        self.context.enable_vertex_attrib_array(index);
        self.check_for_error()
    }

    pub fn gl_buffer_data(
        &self,
        target: GLenum,
        size: u64,
        data: *const std::ffi::c_void,
        usage: GLenum,
    ) -> RafxResult<()> {
        unsafe {
            let slice = std::slice::from_raw_parts(data as *const u8, size as usize);
            self.context.buffer_data_with_u8_array(target, slice, usage);
        }
        self.check_for_error()
    }

    pub fn gl_buffer_sub_data(
        &self,
        target: GLenum,
        offset: u32,
        size: u64,
        data: *const u8,
    ) -> RafxResult<()> {
        unsafe {
            let slice = std::slice::from_raw_parts(data as *const u8, size as usize);
            self.context
                .buffer_sub_data_with_i32_and_u8_array(target, offset as i32, slice);
        }
        self.check_for_error()
    }

    pub fn gl_create_shader(
        &self,
        shader_type: GLenum,
    ) -> RafxResult<ShaderId> {
        let shader = self.context.create_shader(shader_type).unwrap();
        self.check_for_error()?;
        let shader_id = ShaderId(NEXT_GL_SHADER_ID.fetch_add(1, Ordering::Relaxed));
        let old = self.shaders.lock().unwrap().insert(shader_id, shader);
        assert!(old.is_none());
        Ok(shader_id)
    }

    pub fn gl_destroy_shader(
        &self,
        shader_id: ShaderId,
    ) -> RafxResult<()> {
        let shader = self.shaders.lock().unwrap().remove(&shader_id).unwrap();
        self.context.delete_shader(Some(&shader));
        self.check_for_error()
    }

    pub fn gl_shader_source(
        &self,
        shader_id: ShaderId,
        code: &CString,
    ) -> RafxResult<()> {
        let shaders = self.shaders.lock().unwrap();
        self.context
            .shader_source(shaders.get(&shader_id).unwrap(), code.to_str().unwrap());
        self.check_for_error()
    }

    pub fn gl_compile_shader(
        &self,
        shader_id: ShaderId,
    ) -> RafxResult<()> {
        let shaders = self.shaders.lock().unwrap();
        self.context
            .compile_shader(shaders.get(&shader_id).unwrap());
        self.check_for_error()
    }

    pub fn gl_get_shaderiv(
        &self,
        shader_id: ShaderId,
        pname: GLenum,
    ) -> RafxResult<i32> {
        let shaders = self.shaders.lock().unwrap();
        let value = self
            .context
            .get_shader_parameter(shaders.get(&shader_id).unwrap(), pname);
        self.check_for_error()?;
        Ok(convert_js_to_i32(&value).ok_or_else(|| {
            format!(
                "Parameter {} in convert_js_to_i32 is a {:?} which is neither a number or boolean",
                pname, value
            )
        })?)
    }

    pub fn gl_get_programiv(
        &self,
        program_id: ProgramId,
        pname: GLenum,
    ) -> RafxResult<i32> {
        // This pname is undefined in webgl
        if pname == gles2_bindings::ACTIVE_UNIFORM_MAX_LENGTH {
            // Return a
            return Ok(256);
        }

        let programs = self.programs.lock().unwrap();
        let value = self
            .context
            .get_program_parameter(programs.get(&program_id).unwrap(), pname);
        self.check_for_error()?;
        Ok(convert_js_to_i32(&value).ok_or_else(|| {
            format!(
                "Parameter {} in convert_js_to_i32 is a {:?} which is neither a number or boolean",
                pname, value
            )
        })?)
    }

    fn get_shader_info_log(
        &self,
        shader_id: ShaderId,
    ) -> RafxResult<Option<String>> {
        let shaders = self.shaders.lock().unwrap();
        let value = self
            .context
            .get_shader_info_log(shaders.get(&shader_id).unwrap())
            .ok_or("Shader log unavailable")?;

        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }

    pub fn compile_shader(
        &self,
        shader_type: GLenum,
        src: &CString,
    ) -> RafxResult<ShaderId> {
        let shader_id = self.gl_create_shader(shader_type)?;
        self.gl_shader_source(shader_id, &src)?;
        self.gl_compile_shader(shader_id)?;
        if self.gl_get_shaderiv(shader_id, gles2_bindings::COMPILE_STATUS)? == 0 {
            return Err(match self.get_shader_info_log(shader_id)? {
                Some(x) => format!("Error compiling shader: {}", x),
                None => "Error compiling shader, info log not available".to_string(),
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

    pub fn gl_destroy_program(
        &self,
        program_id: ProgramId,
    ) -> RafxResult<()> {
        let program = self.programs.lock().unwrap().remove(&program_id).unwrap();
        self.context.delete_program(Some(&program));
        self.check_for_error()
    }

    pub fn gl_attach_shader(
        &self,
        program_id: ProgramId,
        shader_id: ShaderId,
    ) -> RafxResult<()> {
        let programs = self.programs.lock().unwrap();
        let shaders = self.shaders.lock().unwrap();
        self.context.attach_shader(
            programs.get(&program_id).unwrap(),
            shaders.get(&shader_id).unwrap(),
        );
        self.check_for_error()
    }

    pub fn gl_link_program(
        &self,
        program_id: ProgramId,
    ) -> RafxResult<()> {
        let programs = self.programs.lock().unwrap();
        self.context
            .link_program(programs.get(&program_id).unwrap());
        self.check_for_error()
    }

    pub fn gl_validate_program(
        &self,
        program_id: ProgramId,
    ) -> RafxResult<()> {
        let programs = self.programs.lock().unwrap();
        self.context
            .validate_program(programs.get(&program_id).unwrap());
        self.check_for_error()
    }

    fn get_program_info_log(
        &self,
        program_id: ProgramId,
    ) -> RafxResult<Option<String>> {
        let programs = self.programs.lock().unwrap();
        let value = self
            .context
            .get_program_info_log(programs.get(&program_id).unwrap())
            .ok_or("Program log unavailable")?;

        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }

    pub fn link_shader_program(
        &self,
        program_id: ProgramId,
    ) -> RafxResult<()> {
        self.gl_link_program(program_id)?;
        if self.gl_get_programiv(program_id, gles2_bindings::LINK_STATUS)? == 0 {
            return Err(match self.get_program_info_log(program_id)? {
                Some(x) => format!("Error linking shader program: {}", x),
                None => "Error linking shader program, info log not available".to_string(),
            })?;
        }

        if let Ok(Some(debug_info)) = self.get_program_info_log(program_id) {
            log::debug!("Debug info while linking shader program: {}", debug_info);
        }

        Ok(())
    }

    pub fn validate_shader_program(
        &self,
        program_id: ProgramId,
    ) -> RafxResult<()> {
        self.gl_validate_program(program_id)?;
        if self.gl_get_programiv(program_id, gles2_bindings::VALIDATE_STATUS)? == 0 {
            return Err(match self.get_program_info_log(program_id)? {
                Some(x) => format!("Error validating shader program: {}", x),
                None => "Error validating shader program, info log not available".to_string(),
            })?;
        }

        if let Ok(Some(debug_info)) = self.get_program_info_log(program_id) {
            log::debug!("Debug info while validating shader program: {}", debug_info);
        }

        Ok(())
    }

    pub fn gl_get_uniform_location(
        &self,
        program_id: ProgramId,
        name: &CStr,
    ) -> RafxResult<Option<LocationId>> {
        let programs = self.programs.lock().unwrap();
        let location = self
            .context
            .get_uniform_location(programs.get(&program_id).unwrap(), &*name.to_string_lossy());
        self.check_for_error()?;
        let location = location.map(|x| LocationId(x));
        Ok(location)
    }

    pub fn get_active_uniform_max_name_length_hint(
        &self,
        _program_id: ProgramId,
    ) -> RafxResult<GetActiveUniformMaxNameLengthHint> {
        Ok(GetActiveUniformMaxNameLengthHint)
    }

    pub fn gl_get_active_uniform(
        &self,
        program_id: ProgramId,
        index: u32,
        _max_name_length_hint: &GetActiveUniformMaxNameLengthHint,
    ) -> RafxResult<ActiveUniformInfo> {
        let programs = self.programs.lock().unwrap();
        let info = self
            .context
            .get_active_uniform(programs.get(&program_id).unwrap(), index)
            .ok_or_else(|| format!("Did not find uniform {} in gl_get_active_uniform", index))?;

        Ok(ActiveUniformInfo {
            name: CString::new(info.name()).unwrap(),
            size: info.size() as u32,
            ty: info.type_(),
        })
    }

    pub fn gl_flush(&self) -> RafxResult<()> {
        self.context.flush();
        self.check_for_error()
    }

    pub fn gl_disable(
        &self,
        value: GLenum,
    ) -> RafxResult<()> {
        self.context.disable(value);
        self.check_for_error()
    }

    pub fn gl_enable(
        &self,
        value: GLenum,
    ) -> RafxResult<()> {
        self.context.enable(value);
        self.check_for_error()
    }

    pub fn gl_cull_face(
        &self,
        mode: GLenum,
    ) -> RafxResult<()> {
        self.context.cull_face(mode);
        self.check_for_error()
    }

    pub fn gl_front_face(
        &self,
        mode: GLenum,
    ) -> RafxResult<()> {
        self.context.front_face(mode);
        self.check_for_error()
    }

    pub fn gl_depth_mask(
        &self,
        flag: bool,
    ) -> RafxResult<()> {
        self.context.depth_mask(flag);
        self.check_for_error()
    }

    pub fn gl_depth_func(
        &self,
        value: GLenum,
    ) -> RafxResult<()> {
        self.context.depth_func(value);
        self.check_for_error()
    }

    pub fn gl_stencil_mask(
        &self,
        mask: u32,
    ) -> RafxResult<()> {
        self.context.stencil_mask(mask);
        self.check_for_error()
    }

    pub fn gl_stencil_func_separate(
        &self,
        face: GLenum,
        func: GLenum,
        ref_value: i32,
        mask: GLenum,
    ) -> RafxResult<()> {
        self.context
            .stencil_func_separate(face, func, ref_value, mask);
        self.check_for_error()
    }

    pub fn gl_stencil_op_separate(
        &self,
        face: GLenum,
        sfail: GLenum,
        dpfail: GLenum,
        dppass: GLenum,
    ) -> RafxResult<()> {
        self.context
            .stencil_op_separate(face, sfail, dpfail, dppass);
        self.check_for_error()
    }

    pub fn gl_blend_func_separate(
        &self,
        sfactor_rgb: GLenum,
        dfactor_rgb: GLenum,
        sfactor_alpha: GLenum,
        dfactor_alpha: GLenum,
    ) -> RafxResult<()> {
        self.context
            .blend_func_separate(sfactor_rgb, dfactor_rgb, sfactor_alpha, dfactor_alpha);
        self.check_for_error()
    }

    pub fn gl_blend_equation_separate(
        &self,
        mode_rgb: GLenum,
        mode_alpha: GLenum,
    ) -> RafxResult<()> {
        self.context.blend_equation_separate(mode_rgb, mode_alpha);
        self.check_for_error()
    }

    pub fn gl_bind_attrib_location(
        &self,
        program_id: ProgramId,
        index: u32,
        name: &str,
    ) -> RafxResult<()> {
        let programs = self.programs.lock().unwrap();
        self.context
            .bind_attrib_location(programs.get(&program_id).unwrap(), index, name);
        self.check_for_error()
    }

    pub fn gl_use_program(
        &self,
        program_id: ProgramId,
    ) -> RafxResult<()> {
        if program_id == NONE_PROGRAM {
            self.context.use_program(None);
        } else {
            let programs = self.programs.lock().unwrap();
            self.context
                .use_program(Some(programs.get(&program_id).unwrap()));
        }

        self.check_for_error()
    }

    pub fn gl_bind_renderbuffer(
        &self,
        target: GLenum,
        renderbuffer: RenderbufferId,
    ) -> RafxResult<()> {
        if renderbuffer == NONE_RENDERBUFFER {
            self.context.bind_renderbuffer(target, None);
        } else {
            let renderbuffers = self.renderbuffers.lock().unwrap();
            self.context
                .bind_renderbuffer(target, Some(&renderbuffers.get(&renderbuffer).unwrap()));
        }

        self.check_for_error()
    }

    pub fn gl_framebuffer_renderbuffer(
        &self,
        target: GLenum,
        attachment: GLenum,
        renderbuffer_target: GLenum,
        renderbuffer: RenderbufferId,
    ) -> RafxResult<()> {
        if renderbuffer == NONE_RENDERBUFFER {
            self.context
                .framebuffer_renderbuffer(target, attachment, renderbuffer_target, None);
        } else {
            let renderbuffers = self.renderbuffers.lock().unwrap();
            self.context.framebuffer_renderbuffer(
                target,
                attachment,
                renderbuffer_target,
                Some(&renderbuffers.get(&renderbuffer).unwrap()),
            );
        }

        self.check_for_error()
    }

    pub fn gl_framebuffer_texture(
        &self,
        target: GLenum,
        attachment: GLenum,
        texture_target: GLenum,
        texture_id: TextureId,
        mip_level: u8,
    ) -> RafxResult<()> {
        if texture_id == NONE_TEXTURE {
            self.context.framebuffer_texture_2d(
                target,
                attachment,
                texture_target,
                None,
                mip_level as _,
            );
        } else {
            let textures = self.textures.lock().unwrap();
            self.context.framebuffer_texture_2d(
                target,
                attachment,
                texture_target,
                Some(textures.get(&texture_id).unwrap()),
                mip_level as _,
            );
        }

        self.check_for_error()
    }

    pub fn gl_check_framebuffer_status(
        &self,
        target: GLenum,
    ) -> RafxResult<u32> {
        let result = self.context.check_framebuffer_status(target);
        self.check_for_error()?;
        Ok(result)
    }

    pub fn gl_disable_vertex_attrib_array(
        &self,
        index: u32,
    ) -> RafxResult<()> {
        self.context.disable_vertex_attrib_array(index);
        self.check_for_error()
    }

    pub fn gl_draw_arrays(
        &self,
        mode: GLenum,
        first: i32,
        count: i32,
    ) -> RafxResult<()> {
        self.context.draw_arrays(mode, first, count);
        self.check_for_error()
    }

    pub fn gl_draw_elements(
        &self,
        mode: GLenum,
        count: i32,
        type_: GLenum,
        byte_offset: u32,
    ) -> RafxResult<()> {
        self.context
            .draw_elements_with_i32(mode, count, type_, byte_offset as _);
        self.check_for_error()
    }

    pub fn gl_uniform_1iv<T: Copy>(
        &self,
        location: &LocationId,
        data: &T,
        count: u32,
    ) -> RafxResult<()> {
        unsafe {
            let data_slice = data as *const T as *const i32;
            let slice = std::slice::from_raw_parts(data_slice, count as usize);
            self.context
                .uniform1iv_with_i32_array(Some(&location.0), slice);
            self.check_for_error()
        }
    }

    pub fn gl_uniform_1fv<T: Copy>(
        &self,
        location: &LocationId,
        data: &T,
        count: u32,
    ) -> RafxResult<()> {
        unsafe {
            let data_slice = data as *const T as *const f32;
            let slice = std::slice::from_raw_parts(data_slice, count as usize);
            self.context
                .uniform1fv_with_f32_array(Some(&location.0), slice);
            self.check_for_error()
        }
    }

    pub fn gl_uniform_2iv<T: Copy>(
        &self,
        location: &LocationId,
        data: &T,
        count: u32,
    ) -> RafxResult<()> {
        unsafe {
            let data_slice = data as *const T as *const i32;
            let slice = std::slice::from_raw_parts(data_slice, 2 * count as usize);
            self.context
                .uniform2iv_with_i32_array(Some(&location.0), slice);
            self.check_for_error()
        }
    }

    pub fn gl_uniform_2fv<T: Copy>(
        &self,
        location: &LocationId,
        data: &T,
        count: u32,
    ) -> RafxResult<()> {
        unsafe {
            let data_slice = data as *const T as *const f32;
            let slice = std::slice::from_raw_parts(data_slice, 2 * count as usize);
            self.context
                .uniform2fv_with_f32_array(Some(&location.0), slice);
            self.check_for_error()
        }
    }

    pub fn gl_uniform_3iv<T: Copy>(
        &self,
        location: &LocationId,
        data: &T,
        count: u32,
    ) -> RafxResult<()> {
        unsafe {
            let data_slice = data as *const T as *const i32;
            let slice = std::slice::from_raw_parts(data_slice, 3 * count as usize);
            self.context
                .uniform3iv_with_i32_array(Some(&location.0), slice);
            self.check_for_error()
        }
    }

    pub fn gl_uniform_3fv<T: Copy>(
        &self,
        location: &LocationId,
        data: &T,
        count: u32,
    ) -> RafxResult<()> {
        unsafe {
            let data_slice = data as *const T as *const f32;
            let slice = std::slice::from_raw_parts(data_slice, 3 * count as usize);
            self.context
                .uniform3fv_with_f32_array(Some(&location.0), slice);
            self.check_for_error()
        }
    }

    pub fn gl_uniform_4iv<T: Copy>(
        &self,
        location: &LocationId,
        data: &T,
        count: u32,
    ) -> RafxResult<()> {
        unsafe {
            let data_slice = data as *const T as *const i32;
            let slice = std::slice::from_raw_parts(data_slice, 4 * count as usize);
            self.context
                .uniform4iv_with_i32_array(Some(&location.0), slice);
            self.check_for_error()
        }
    }

    pub fn gl_uniform_4fv<T: Copy>(
        &self,
        location: &LocationId,
        data: &T,
        count: u32,
    ) -> RafxResult<()> {
        unsafe {
            let data_slice = data as *const T as *const f32;
            let slice = std::slice::from_raw_parts(data_slice, 4 * count as usize);
            self.context
                .uniform4fv_with_f32_array(Some(&location.0), slice);
            self.check_for_error()
        }
    }

    pub fn gl_uniform_matrix_2fv<T: Copy>(
        &self,
        location: &LocationId,
        data: &T,
        count: u32,
    ) -> RafxResult<()> {
        unsafe {
            let data_slice = data as *const T as *const f32;
            let slice = std::slice::from_raw_parts(data_slice, 16 * count as usize);
            self.context
                .uniform_matrix2fv_with_f32_array(Some(&location.0), false, slice);
            self.check_for_error()
        }
    }

    pub fn gl_uniform_matrix_3fv<T: Copy>(
        &self,
        location: &LocationId,
        data: &T,
        count: u32,
    ) -> RafxResult<()> {
        unsafe {
            let data_slice = data as *const T as *const f32;
            let slice = std::slice::from_raw_parts(data_slice, 24 * count as usize);
            self.context
                .uniform_matrix3fv_with_f32_array(Some(&location.0), false, slice);
            self.check_for_error()
        }
    }

    pub fn gl_uniform_matrix_4fv<T: Copy>(
        &self,
        location: &LocationId,
        data: &T,
        count: u32,
    ) -> RafxResult<()> {
        unsafe {
            let data_slice = data as *const T as *const f32;
            let slice = std::slice::from_raw_parts(data_slice, 32 * count as usize);
            self.context
                .uniform_matrix4fv_with_f32_array(Some(&location.0), false, slice);
            self.check_for_error()
        }
    }

    pub fn gl_active_texture(
        &self,
        i: u32
    ) -> RafxResult<()> {
        unsafe {
            self.context.active_texture(gles2_bindings::TEXTURE0 + i);
            self.check_for_error()
        }
    }

    pub fn gl_bind_texture(
        &self,
        target: GLenum,
        texture_id: TextureId,
    ) -> RafxResult<()> {
        let textures = self.textures.lock().unwrap();
        self.context.bind_texture(target, textures.get(&texture_id));
        self.check_for_error()
    }

    pub fn gl_tex_image_2d(
        &self,
        target: GLenum,
        mip_level: u8,
        internal_format: i32,
        width: u32,
        height: u32,
        border: i32,
        format: GLenum,
        type_: u32,
        pixels: Option<&[u8]>,
    ) -> RafxResult<()> {
        self.context
            .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                target,
                mip_level as _,
                internal_format,
                width as _,
                height as _,
                border,
                format,
                type_,
                pixels,
            )
            .map_err(|x| format!("{:?}", x))?;
        self.check_for_error()
    }

    pub fn gl_tex_parameteri(&self, target: GLenum, pname: GLenum, param: i32) -> RafxResult<()> {
        unsafe {
            self.context.tex_parameteri(target, pname, param);
            self.check_for_error()
        }
    }
}
