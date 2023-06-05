use legion::Resources;

pub use crate::daemon_args::AssetDaemonArgs;
use crate::time::TimeState;
use rafx::assets::distill_impl::AssetResource;

#[cfg(feature = "egui")]
use rafx_plugins::features::egui::EguiContextResource;

#[cfg(feature = "basic-pipeline")]
use rafx_plugins::pipelines::basic::AntiAliasMethodBasic as AntiAliasMethod;
#[cfg(feature = "basic-pipeline")]
use rafx_plugins::pipelines::basic::TonemapperTypeBasic as TonemapperType;

#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::pipelines::modern::AntiAliasMethodAdv as AntiAliasMethod;
#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::pipelines::modern::ModernPipelineMeshCullingDebugData;
#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::pipelines::modern::ModernPipelineTonemapDebugData;
#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::pipelines::modern::TonemapperTypeAdv as TonemapperType;
#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::pipelines::modern::{JitterPattern, TemporalAAOptions};

#[derive(Clone)]
pub struct RenderOptions {
    pub anti_alias_method: AntiAliasMethod,
    pub enable_hdr: bool,
    #[cfg(not(feature = "basic-pipeline"))]
    pub enable_ssao: bool,
    pub enable_bloom: bool,
    pub enable_textures: bool,
    pub enable_lighting: bool,
    pub show_surfaces: bool,
    pub show_wireframes: bool,
    pub show_debug3d: bool,
    pub show_text: bool,
    pub show_skybox: bool,
    pub show_feature_toggles: bool,
    pub show_shadows: bool,
    pub show_lights_debug_draw: bool,
    pub blur_pass_count: usize,
    pub tonemapper_type: TonemapperType,
    pub enable_visibility_update: bool,
    pub use_clustered_lighting: bool,
    #[cfg(not(feature = "basic-pipeline"))]
    pub ndf_filter_amount: f32,
    #[cfg(not(feature = "basic-pipeline"))]
    pub taa_options: TemporalAAOptions,
    #[cfg(not(feature = "basic-pipeline"))]
    pub enable_sharpening: bool,
    #[cfg(not(feature = "basic-pipeline"))]
    pub sharpening_amount: f32,
    #[cfg(not(feature = "basic-pipeline"))]
    pub enable_occlusion_culling: bool,
}

impl RenderOptions {
    pub fn default_2d() -> Self {
        RenderOptions {
            anti_alias_method: AntiAliasMethod::None,
            enable_hdr: false,
            #[cfg(not(feature = "basic-pipeline"))]
            enable_ssao: false,
            enable_bloom: false,
            enable_textures: true,
            enable_lighting: true,
            show_surfaces: true,
            show_wireframes: false,
            show_debug3d: true,
            show_text: true,
            show_skybox: false,
            show_shadows: true,
            show_feature_toggles: false,
            show_lights_debug_draw: false,
            blur_pass_count: 0,
            tonemapper_type: TonemapperType::None,
            enable_visibility_update: true,
            use_clustered_lighting: true,
            #[cfg(not(feature = "basic-pipeline"))]
            ndf_filter_amount: 1.0,
            #[cfg(not(feature = "basic-pipeline"))]
            taa_options: Default::default(),
            #[cfg(not(feature = "basic-pipeline"))]
            enable_sharpening: false,
            #[cfg(not(feature = "basic-pipeline"))]
            sharpening_amount: 0.0,
            #[cfg(not(feature = "basic-pipeline"))]
            enable_occlusion_culling: false,
        }
    }

    pub fn default_3d() -> Self {
        RenderOptions {
            anti_alias_method: AntiAliasMethod::default(),
            enable_hdr: true,
            #[cfg(not(feature = "basic-pipeline"))]
            enable_ssao: true,
            enable_bloom: true,
            enable_textures: true,
            enable_lighting: true,
            show_surfaces: true,
            show_wireframes: false,
            show_debug3d: true,
            show_text: true,
            show_skybox: true,
            show_shadows: true,
            show_feature_toggles: true,
            show_lights_debug_draw: false,
            blur_pass_count: 5,
            tonemapper_type: TonemapperType::default(),
            enable_visibility_update: true,
            use_clustered_lighting: true,
            #[cfg(not(feature = "basic-pipeline"))]
            ndf_filter_amount: 1.0,
            #[cfg(not(feature = "basic-pipeline"))]
            taa_options: Default::default(),
            #[cfg(not(feature = "basic-pipeline"))]
            enable_sharpening: true,
            #[cfg(not(feature = "basic-pipeline"))]
            sharpening_amount: 1.0,
            #[cfg(not(feature = "basic-pipeline"))]
            enable_occlusion_culling: true,
        }
    }
}

impl RenderOptions {
    #[cfg(feature = "egui")]
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
    ) {
        let aa_method_names: Vec<_> = (0..(AntiAliasMethod::MAX as i32))
            .map(|t| AntiAliasMethod::from(t).display_name())
            .collect();

        egui::ComboBox::from_label("anti_alias_method")
            .selected_text(aa_method_names[self.anti_alias_method as usize])
            .show_ui(ui, |ui| {
                for (i, name) in aa_method_names.iter().enumerate() {
                    ui.selectable_value(
                        &mut self.anti_alias_method,
                        AntiAliasMethod::from(i as i32),
                        name,
                    );
                }
            });

        ui.checkbox(&mut self.enable_hdr, "enable_hdr");

        if self.enable_hdr {
            ui.indent("HDR options", |ui| {
                let tonemapper_names: Vec<_> = (0..(TonemapperType::MAX as i32))
                    .map(|t| TonemapperType::from(t).display_name())
                    .collect();

                egui::ComboBox::from_label("tonemapper_type")
                    .selected_text(tonemapper_names[self.tonemapper_type as usize])
                    .show_ui(ui, |ui| {
                        for (i, name) in tonemapper_names.iter().enumerate() {
                            ui.selectable_value(
                                &mut self.tonemapper_type,
                                TonemapperType::from(i as i32),
                                name,
                            );
                        }
                    });

                ui.checkbox(&mut self.enable_bloom, "enable_bloom");
                if self.enable_bloom {
                    ui.indent("", |ui| {
                        ui.add(
                            egui::Slider::new(&mut self.blur_pass_count, 0..=10)
                                .clamp_to_range(true)
                                .text("blur_pass_count"),
                        );
                    });
                }
            });
        }

        #[cfg(not(feature = "basic-pipeline"))]
        if self.anti_alias_method != AntiAliasMethod::Msaa4x {
            ui.checkbox(&mut self.enable_ssao, "enable_ssao");
        }

        ui.checkbox(&mut self.show_lights_debug_draw, "show_lights_debug_draw");
        ui.checkbox(&mut self.use_clustered_lighting, "use_clustered_lighting");

        if self.show_feature_toggles {
            ui.checkbox(&mut self.show_wireframes, "show_wireframes");
            ui.checkbox(&mut self.show_surfaces, "show_surfaces");

            if self.show_surfaces {
                ui.indent("", |ui| {
                    ui.checkbox(&mut self.enable_textures, "enable_textures");
                    ui.checkbox(&mut self.enable_lighting, "enable_lighting");

                    if self.enable_lighting {
                        ui.indent("", |ui| {
                            ui.checkbox(&mut self.show_shadows, "show_shadows");
                        });
                    }

                    ui.checkbox(&mut self.show_skybox, "show_skybox_feature");
                });
            }

            ui.checkbox(&mut self.show_debug3d, "show_debug3d_feature");
            ui.checkbox(&mut self.show_text, "show_text_feature");
        }

        ui.checkbox(
            &mut self.enable_visibility_update,
            "enable_visibility_update",
        );

        #[cfg(not(feature = "basic-pipeline"))]
        ui.add(egui::Slider::new(&mut self.ndf_filter_amount, 0.0..=4.0).text("ndf_filter_amount"));

        #[cfg(not(feature = "basic-pipeline"))]
        ui.checkbox(&mut self.enable_sharpening, "enable_sharpening");
        #[cfg(not(feature = "basic-pipeline"))]
        ui.add(egui::Slider::new(&mut self.sharpening_amount, 0.0..=1.0).text("sharpening_amount"));
        #[cfg(not(feature = "basic-pipeline"))]
        ui.checkbox(
            &mut self.enable_occlusion_culling,
            "enable_occlusion_culling",
        );
    }
}

#[derive(Default)]
pub struct DebugUiState {
    pub show_render_options: bool,
    pub show_asset_list: bool,
    #[cfg(not(feature = "basic-pipeline"))]
    pub show_taa_options: bool,
    #[cfg(not(feature = "basic-pipeline"))]
    pub show_tonemap_debug: bool,
    #[cfg(not(feature = "basic-pipeline"))]
    pub show_mesh_culling_debug: bool,

    #[cfg(feature = "profile-with-puffin")]
    pub show_profiler: bool,
}

#[cfg(not(feature = "basic-pipeline"))]
pub fn draw_taa_options(
    ui: &mut egui::Ui,
    render_options: &mut RenderOptions,
) {
    let taa_options = &mut render_options.taa_options;
    ui.checkbox(
        &mut taa_options.enable_side_by_side_debug_view,
        "side_by_side_debug_view",
    );
    ui.add(
        egui::Slider::new(&mut taa_options.forward_pass_mip_bias, -5.0..=5.0)
            .text("forward_pass_mip_bias"),
    );

    ui.add(
        egui::Slider::new(&mut taa_options.jitter_multiplier, 0.0..=3.0).text("jitter_multiplier"),
    );
    ui.add(egui::Slider::new(&mut taa_options.history_weight, 0.0..=1.0).text("history_weight"));
    ui.add(
        egui::Slider::new(
            &mut taa_options.history_weight_velocity_adjust_multiplier,
            0.0..=100.0,
        )
        .text("velocity_weight_adjust_multiplier"),
    );
    ui.add(
        egui::Slider::new(
            &mut taa_options.history_weight_velocity_adjust_max,
            0.0..=1.0,
        )
        .text("velocity_weight_adjust_max"),
    );

    let jitter_pattern_names: Vec<_> = (0..(JitterPattern::MAX as i32))
        .map(|t| JitterPattern::from(t).display_name())
        .collect();

    egui::ComboBox::from_label("jitter_pattern")
        .selected_text(jitter_pattern_names[taa_options.jitter_pattern as usize])
        .show_ui(ui, |ui| {
            for (i, name) in jitter_pattern_names.iter().enumerate() {
                ui.selectable_value(
                    &mut taa_options.jitter_pattern,
                    JitterPattern::from(i as i32),
                    name,
                );
            }
        });
}

#[cfg(feature = "egui")]
pub fn draw_ui(resources: &Resources) {
    let ctx = resources.get::<EguiContextResource>().unwrap().context();
    let time_state = resources.get::<TimeState>().unwrap();
    let mut debug_ui_state = resources.get_mut::<DebugUiState>().unwrap();
    let mut render_options = resources.get_mut::<RenderOptions>().unwrap();
    #[cfg(not(feature = "basic-pipeline"))]
    let tonemap_debug_data = resources.get::<ModernPipelineTonemapDebugData>().unwrap();
    #[cfg(not(feature = "basic-pipeline"))]
    let mesh_culling_debug_data = resources
        .get::<ModernPipelineMeshCullingDebugData>()
        .unwrap();
    let asset_resource = resources.get::<AssetResource>().unwrap();

    egui::TopBottomPanel::top("top_panel").show(&ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            egui::menu::menu(ui, "Windows", |ui| {
                ui.checkbox(&mut debug_ui_state.show_render_options, "Render Options");

                #[cfg(not(feature = "basic-pipeline"))]
                ui.checkbox(&mut debug_ui_state.show_taa_options, "TAA Options");

                ui.checkbox(&mut debug_ui_state.show_asset_list, "Asset List");

                #[cfg(not(feature = "basic-pipeline"))]
                ui.checkbox(&mut debug_ui_state.show_tonemap_debug, "Tonemap Debug");

                #[cfg(not(feature = "basic-pipeline"))]
                ui.checkbox(
                    &mut debug_ui_state.show_mesh_culling_debug,
                    "Mesh Culling Debug",
                );

                #[cfg(feature = "profile-with-puffin")]
                if ui
                    .checkbox(&mut debug_ui_state.show_profiler, "Profiler")
                    .changed()
                {
                    log::info!(
                        "Setting puffin profiler enabled: {:?}",
                        debug_ui_state.show_profiler
                    );
                    profiling::puffin::set_scopes_on(debug_ui_state.show_profiler);
                }
            });

            ui.with_layout(egui::Layout::right_to_left(), |ui| {
                ui.label(format!("Frame: {}", time_state.update_count()));
                ui.separator();
                ui.label(format!(
                    "FPS: {:.1}",
                    time_state.updates_per_second_smoothed()
                ));
            });
        })
    });

    #[cfg(not(feature = "basic-pipeline"))]
    if debug_ui_state.show_tonemap_debug {
        egui::Window::new("Tonemap Debug")
            .open(&mut debug_ui_state.show_tonemap_debug)
            .show(&ctx, |ui| {
                let data = tonemap_debug_data.inner.lock().unwrap();

                ui.add(egui::Label::new(format!(
                    "histogram_sample_count: {}",
                    data.histogram_sample_count
                )));
                ui.add(egui::Label::new(format!(
                    "histogram_max_value: {}",
                    data.histogram_max_value
                )));

                use egui::plot::{Line, Plot, VLine, Value, Values};
                let line_values: Vec<_> = data
                    .histogram
                    .iter()
                    //.skip(1) // don't include index 0
                    .enumerate()
                    .map(|(i, value)| Value::new(i as f64, *value as f64))
                    .collect();
                let line = Line::new(Values::from_values_iter(line_values.into_iter())).fill(0.0);
                let average_line = VLine::new(data.result_average_bin);
                let low_line = VLine::new(data.result_low_bin);
                let high_line = VLine::new(data.result_high_bin);
                Some(
                    ui.add(
                        Plot::new("my_plot")
                            .line(line)
                            .vline(average_line)
                            .vline(low_line)
                            .vline(high_line)
                            .include_y(0.0)
                            .include_y(1.0)
                            .show_axes([false, false]),
                    ),
                )
            });
    }

    #[cfg(not(feature = "basic-pipeline"))]
    if debug_ui_state.show_mesh_culling_debug {
        egui::Window::new("Mesh Culling Debug")
            .open(&mut debug_ui_state.show_mesh_culling_debug)
            .show(&ctx, |ui| {
                let data = mesh_culling_debug_data.inner.lock().unwrap();

                ui.add(egui::Label::new(format!(
                    "culled_mesh_count: {}",
                    data.culled_mesh_count
                )));
                ui.add(egui::Label::new(format!(
                    "total_mesh_count: {}",
                    data.total_mesh_count
                )));

                ui.add(egui::Label::new(format!(
                    "culled_primitive_count: {}",
                    data.culled_primitive_count
                )));
                ui.add(egui::Label::new(format!(
                    "total_primitive_count: {}",
                    data.total_primitive_count
                )));
            });
    }

    #[cfg(not(feature = "basic-pipeline"))]
    {
        tonemap_debug_data
            .inner
            .lock()
            .unwrap()
            .enable_debug_data_collection = debug_ui_state.show_tonemap_debug;
    }

    #[cfg(not(feature = "basic-pipeline"))]
    {
        mesh_culling_debug_data
            .inner
            .lock()
            .unwrap()
            .enable_debug_data_collection = debug_ui_state.show_mesh_culling_debug;
    }

    if debug_ui_state.show_render_options {
        egui::Window::new("Render Options")
            .open(&mut debug_ui_state.show_render_options)
            .show(&ctx, |ui| {
                render_options.ui(ui);
            });
    }

    #[cfg(not(feature = "basic-pipeline"))]
    if debug_ui_state.show_taa_options {
        egui::Window::new("TAA Options")
            .open(&mut debug_ui_state.show_taa_options)
            .show(&ctx, |ui| {
                draw_taa_options(ui, &mut *render_options);
            });
    }

    if debug_ui_state.show_asset_list {
        egui::Window::new("Asset List")
            .open(&mut debug_ui_state.show_asset_list)
            .show(&ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let loader = asset_resource.loader();
                    let mut asset_info = loader
                        .get_active_loads()
                        .into_iter()
                        .map(|item| loader.get_load_info(item))
                        .collect::<Vec<_>>();
                    asset_info.sort_by(|x, y| {
                        x.as_ref()
                            .map(|x| &x.path)
                            .cmp(&y.as_ref().map(|y| &y.path))
                    });
                    for info in asset_info {
                        if let Some(info) = info {
                            let id = info.asset_id;
                            ui.label(format!(
                                "{}:{} .. {}",
                                info.file_name.unwrap_or_else(|| "???".to_string()),
                                info.asset_name.unwrap_or_else(|| format!("{}", id)),
                                info.refs
                            ));
                        } else {
                            ui.label("NO INFO");
                        }
                    }
                });
            });
    }

    #[cfg(feature = "profile-with-puffin")]
    if debug_ui_state.show_profiler {
        profiling::scope!("puffin profiler");
        puffin_egui::profiler_window(&ctx);
    }
}
