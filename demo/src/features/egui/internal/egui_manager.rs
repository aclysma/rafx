use super::EguiContextResource;
use super::EguiDrawData;
use egui::{CtxRef, FontDefinitions};
use std::sync::Arc;
use std::sync::Mutex;

// Inner state for EguiManager, which will be protected by a Mutex. Mutex protection required since
// this object is Send but not Sync
struct EguiManagerInner {
    context: egui::CtxRef,
    raw_input: egui::RawInput,
    start_time: rafx::base::Instant,

    // This is produced when calling render()
    font_atlas: Option<Arc<egui::Texture>>,
    clipped_meshes: Option<Vec<egui::epaint::ClippedMesh>>,
}

//TODO: Investigate usage of channels/draw lists
#[derive(Clone)]
pub struct EguiManager {
    inner: Arc<Mutex<EguiManagerInner>>,
}

// Wraps egui (and winit integration logic)
impl EguiManager {
    // egui and winit platform are expected to be pre-configured
    pub fn new() -> Self {
        let ctx = CtxRef::default();
        let mut font_definitions = FontDefinitions::default();

        // Can remove the default_fonts feature and use custom fonts instead
        font_definitions.font_data.insert(
            "mplus-1p".to_string(),
            std::borrow::Cow::Borrowed(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/fonts/mplus-1p-regular.ttf"
            ))),
        );
        font_definitions.font_data.insert(
            "feather".to_string(),
            std::borrow::Cow::Borrowed(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/fonts/feather.ttf"
            ))),
        );
        font_definitions.font_data.insert(
            "materialdesignicons".to_string(),
            std::borrow::Cow::Borrowed(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/fonts/materialdesignicons-webfont.ttf"
            ))),
        );

        font_definitions.fonts_for_family.insert(
            egui::FontFamily::Monospace,
            vec![
                "mplus-1p".to_owned(),
                "feather".to_owned(), // fallback for √ etc
                "materialdesignicons".to_owned(),
            ],
        );
        font_definitions.fonts_for_family.insert(
            egui::FontFamily::Proportional,
            vec![
                "mplus-1p".to_owned(),
                "feather".to_owned(),
                "materialdesignicons".to_owned(),
            ],
        );

        font_definitions.family_and_size.insert(
            egui::TextStyle::Small,
            (egui::FontFamily::Proportional, 12.0),
        );
        font_definitions.family_and_size.insert(
            egui::TextStyle::Body,
            (egui::FontFamily::Proportional, 14.0),
        );
        font_definitions.family_and_size.insert(
            egui::TextStyle::Button,
            (egui::FontFamily::Proportional, 16.0),
        );
        font_definitions.family_and_size.insert(
            egui::TextStyle::Heading,
            (egui::FontFamily::Proportional, 20.0),
        );
        font_definitions.family_and_size.insert(
            egui::TextStyle::Monospace,
            (egui::FontFamily::Monospace, 12.0),
        );

        ctx.set_fonts(font_definitions);
        ctx.set_style(egui::Style::default());

        let raw_input = egui::RawInput::default();

        EguiManager {
            inner: Arc::new(Mutex::new(EguiManagerInner {
                context: ctx,
                raw_input,
                start_time: rafx::base::Instant::now(),
                font_atlas: None,
                clipped_meshes: None,
            })),
        }
    }

    pub fn context_resource(&self) -> EguiContextResource {
        EguiContextResource {
            egui_manager: self.clone(),
        }
    }

    // Start a new frame
    #[profiling::function]
    pub fn begin_frame(
        &self,
        screen_width: u32,
        screen_height: u32,
        pixels_per_point: f32,
    ) {
        let mut inner_mutex_guard = self.inner.lock().unwrap();
        let inner = &mut *inner_mutex_guard;

        inner.raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::Vec2::new(screen_width as f32, screen_height as f32),
        ));
        inner.raw_input.pixels_per_point = Some(pixels_per_point);
        inner.raw_input.time = Some(inner.start_time.elapsed().as_secs_f64());

        inner.context.begin_frame(inner.raw_input.take());
    }

    #[profiling::function]
    pub fn end_frame(&self) -> egui::Output {
        let mut inner = self.inner.lock().unwrap();
        let (output, clipped_shapes) = inner.context.end_frame();

        let clipped_meshes = inner.context.tessellate(clipped_shapes);

        //inner.output = Some(output);
        inner.clipped_meshes = Some(clipped_meshes);

        let mut new_texture = None;
        if let Some(texture) = &inner.font_atlas {
            if texture.version != inner.context.texture().version {
                new_texture = Some(inner.context.texture().clone());
            }
        } else {
            new_texture = Some(inner.context.texture().clone());
        }

        if new_texture.is_some() {
            inner.font_atlas = new_texture.clone();
        }

        output
    }

    // Allows access to the context without caller needing to be aware of locking
    pub fn with_context<F>(
        &self,
        f: F,
    ) where
        F: FnOnce(&mut egui::CtxRef),
    {
        let mut guard = self.inner.lock().unwrap();
        let inner = &mut *guard;
        (f)(&mut inner.context);
    }

    pub fn context(&self) -> egui::CtxRef {
        let guard = self.inner.lock().unwrap();
        guard.context.clone()
    }

    pub fn with_context_and_input<F>(
        &self,
        f: F,
    ) where
        F: FnOnce(&mut egui::CtxRef, &mut egui::RawInput),
    {
        let mut guard = self.inner.lock().unwrap();
        let inner = &mut *guard;
        (f)(&mut inner.context, &mut inner.raw_input);
    }

    #[profiling::function]
    pub fn take_draw_data(&self) -> Option<EguiDrawData> {
        let mut inner = self.inner.lock().unwrap();

        let clipped_meshes = inner.clipped_meshes.take();

        EguiDrawData::try_create_new(
            clipped_meshes?,
            inner.font_atlas.as_ref()?.clone(),
            inner.raw_input.pixels_per_point.unwrap_or(1.0),
        )
    }
}
