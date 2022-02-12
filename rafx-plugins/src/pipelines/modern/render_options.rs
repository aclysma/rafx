// Should be kept in sync with the constants in bloom_combine.frag prefixed with OUTPUT_COLOR_SPACE_
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub enum ModernPipelineOutputColorSpace {
    Srgb,
    P3,
}

// Should be kept in sync with the constants in tonemapper.glsl prefixed with TM_
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub enum TonemapperTypeAdv {
    None,
    StephenHillACES,
    SimplifiedLumaACES,
    Hejl2015,
    Hable,
    FilmicALU,
    LogDerivative,
    VisualizeRGBMax,
    VisualizeLuma,
    AutoExposureOld,
    Bergstrom,
    MAX,
}

impl Default for TonemapperTypeAdv {
    fn default() -> Self {
        TonemapperTypeAdv::Bergstrom
    }
}

impl TonemapperTypeAdv {
    pub fn display_name(&self) -> &'static str {
        match self {
            TonemapperTypeAdv::None => "None",
            TonemapperTypeAdv::StephenHillACES => "Stephen Hill ACES",
            TonemapperTypeAdv::SimplifiedLumaACES => "SimplifiedLumaACES",
            TonemapperTypeAdv::Hejl2015 => "Hejl 2015",
            TonemapperTypeAdv::Hable => "Hable",
            TonemapperTypeAdv::FilmicALU => "Filmic ALU (Hable)",
            TonemapperTypeAdv::LogDerivative => "LogDerivative",
            TonemapperTypeAdv::VisualizeRGBMax => "Visualize RGB Max",
            TonemapperTypeAdv::VisualizeLuma => "Visualize RGB Luma",
            TonemapperTypeAdv::AutoExposureOld => "Autoexposure Old",
            TonemapperTypeAdv::Bergstrom => "Bergstrom",
            TonemapperTypeAdv::MAX => "MAX_TONEMAPPER_VALUE",
        }
    }
}

impl From<i32> for TonemapperTypeAdv {
    fn from(v: i32) -> Self {
        assert!(v <= Self::MAX as i32);
        unsafe { std::mem::transmute(v) }
    }
}

impl std::fmt::Display for TonemapperTypeAdv {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// Should be kept in sync with the constants in taa_jitter.glsl prefixed with JP_
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub enum JitterPattern {
    SobolOwen16,
    SobolOwen64,
    Halton,
    QuadJitterTest,
    MAX,
}

impl Default for JitterPattern {
    fn default() -> Self {
        JitterPattern::SobolOwen16
    }
}

impl JitterPattern {
    pub fn display_name(&self) -> &'static str {
        match self {
            JitterPattern::SobolOwen16 => "SobolOwen 16",
            JitterPattern::SobolOwen64 => "SobolOwen 64",
            JitterPattern::Halton => "Halton",
            JitterPattern::QuadJitterTest => "QuadJitterTest",
            JitterPattern::MAX => "JitterPattern MAX VALUE",
        }
    }
}

impl From<i32> for JitterPattern {
    fn from(v: i32) -> Self {
        assert!(v <= Self::MAX as i32);
        unsafe { std::mem::transmute(v) }
    }
}

#[derive(Clone)]
pub struct TemporalAAOptions {
    pub enable_side_by_side_debug_view: bool,
    pub jitter_pattern: JitterPattern,
    pub jitter_multiplier: f32,
    pub history_weight: f32,
    pub history_weight_velocity_adjust_multiplier: f32,
    pub history_weight_velocity_adjust_max: f32,
    pub forward_pass_mip_bias: f32,
}

impl Default for TemporalAAOptions {
    fn default() -> Self {
        TemporalAAOptions {
            enable_side_by_side_debug_view: false,
            jitter_pattern: JitterPattern::SobolOwen16,
            jitter_multiplier: 0.3,
            history_weight: 0.01,
            history_weight_velocity_adjust_multiplier: 50.0,
            history_weight_velocity_adjust_max: 0.10,
            forward_pass_mip_bias: -0.5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub enum AntiAliasMethodAdv {
    None,
    Msaa4x,
    Taa,
    MAX,
}

impl Default for AntiAliasMethodAdv {
    fn default() -> Self {
        AntiAliasMethodAdv::Taa
    }
}

impl AntiAliasMethodAdv {
    pub fn display_name(&self) -> &'static str {
        match self {
            AntiAliasMethodAdv::None => "None",
            AntiAliasMethodAdv::Msaa4x => "4x MSAA",
            AntiAliasMethodAdv::Taa => "TAA",
            AntiAliasMethodAdv::MAX => "AntiAliasMethodAdv MAX VALUE",
        }
    }
}

impl From<i32> for AntiAliasMethodAdv {
    fn from(v: i32) -> Self {
        assert!(v <= Self::MAX as i32);
        unsafe { std::mem::transmute(v) }
    }
}

#[derive(Clone)]
pub struct ModernPipelineRenderOptions {
    pub anti_alias_method: AntiAliasMethodAdv,
    pub taa_options: TemporalAAOptions,
    pub enable_hdr: bool,
    pub enable_ssao: bool,
    pub enable_bloom: bool,
    pub enable_textures: bool,
    pub show_surfaces: bool,
    pub show_wireframes: bool,
    pub show_debug3d: bool,
    pub show_text: bool,
    pub show_skybox: bool,
    pub show_feature_toggles: bool,
    pub blur_pass_count: usize,
    pub tonemapper_type: TonemapperTypeAdv,
    pub enable_visibility_update: bool,
    pub enable_sharpening: bool,
    pub sharpening_amount: f32,
    pub enable_occlusion_culling: bool,
}

impl Default for ModernPipelineRenderOptions {
    fn default() -> Self {
        ModernPipelineRenderOptions {
            anti_alias_method: AntiAliasMethodAdv::Taa,
            taa_options: TemporalAAOptions::default(),
            enable_hdr: true,
            enable_ssao: true,
            enable_bloom: true,
            enable_textures: true,
            show_surfaces: true,
            show_wireframes: false,
            show_debug3d: true,
            show_text: true,
            show_skybox: true,
            show_feature_toggles: true,
            blur_pass_count: 5,
            tonemapper_type: TonemapperTypeAdv::LogDerivative,
            enable_visibility_update: true,
            enable_sharpening: true,
            sharpening_amount: 1.0,
            enable_occlusion_culling: true,
        }
    }
}
