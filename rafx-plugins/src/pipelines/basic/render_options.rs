// Should be kept in sync with the constants in bloom_combine.frag prefixed with OUTPUT_COLOR_SPACE_
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub enum OutputColorSpace {
    Srgb,
    P3,
}

// Should be kept in sync with the constants in tonemapper.glsl prefixed with TM_
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub enum TonemapperType {
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
impl TonemapperType {
    pub fn display_name(&self) -> &'static str {
        match self {
            TonemapperType::None => "None",
            TonemapperType::StephenHillACES => "Stephen Hill ACES",
            TonemapperType::SimplifiedLumaACES => "SimplifiedLumaACES",
            TonemapperType::Hejl2015 => "Hejl 2015",
            TonemapperType::Hable => "Hable",
            TonemapperType::FilmicALU => "Filmic ALU (Hable)",
            TonemapperType::LogDerivative => "LogDerivative",
            TonemapperType::VisualizeRGBMax => "Visualize RGB Max",
            TonemapperType::VisualizeLuma => "Visualize RGB Luma",
            TonemapperType::AutoExposureOld => "Autoexposure Old",
            TonemapperType::Bergstrom => "Bergstrom",
            TonemapperType::MAX => "MAX_TONEMAPPER_VALUE",
        }
    }
}
impl From<i32> for TonemapperType {
    fn from(v: i32) -> Self {
        assert!(v <= Self::MAX as i32);
        unsafe { std::mem::transmute(v) }
    }
}

impl std::fmt::Display for TonemapperType {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[derive(Clone)]
pub struct BasicPipelineRenderOptions {
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
    pub tonemapper_type: TonemapperType,
    pub enable_visibility_update: bool,
}

impl Default for BasicPipelineRenderOptions {
    fn default() -> Self {
        BasicPipelineRenderOptions {
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
            tonemapper_type: TonemapperType::LogDerivative,
            enable_visibility_update: true,
        }
    }
}
