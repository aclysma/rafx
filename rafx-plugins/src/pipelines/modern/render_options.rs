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

#[derive(Clone)]
pub struct ModernPipelineRenderOptions {
    pub enable_msaa: bool,
    pub enable_hdr: bool,
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
}

impl Default for ModernPipelineRenderOptions {
    fn default() -> Self {
        ModernPipelineRenderOptions {
            enable_msaa: true,
            enable_hdr: true,
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
        }
    }
}
