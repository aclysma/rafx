// Should be kept in sync with the constants in bloom_combine.frag prefixed with OUTPUT_COLOR_SPACE_
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub enum BasicPipelineOutputColorSpace {
    Srgb,
    P3,
}

// Should be kept in sync with the constants in tonemapper.glsl prefixed with TM_
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub enum TonemapperTypeBasic {
    None,
    StephenHillACES,
    SimplifiedLumaACES,
    Hejl2015,
    Hable,
    FilmicALU,
    LogDerivative,
    VisualizeRGBMax,
    VisualizeLuma,
    MAX,
}

impl Default for TonemapperTypeBasic {
    fn default() -> Self {
        TonemapperTypeBasic::LogDerivative
    }
}

impl TonemapperTypeBasic {
    pub fn display_name(&self) -> &'static str {
        match self {
            TonemapperTypeBasic::None => "None",
            TonemapperTypeBasic::StephenHillACES => "Stephen Hill ACES",
            TonemapperTypeBasic::SimplifiedLumaACES => "SimplifiedLumaACES",
            TonemapperTypeBasic::Hejl2015 => "Hejl 2015",
            TonemapperTypeBasic::Hable => "Hable",
            TonemapperTypeBasic::FilmicALU => "Filmic ALU (Hable)",
            TonemapperTypeBasic::LogDerivative => "LogDerivative",
            TonemapperTypeBasic::VisualizeRGBMax => "Visualize RGB Max",
            TonemapperTypeBasic::VisualizeLuma => "Visualize RGB Luma",
            TonemapperTypeBasic::MAX => "MAX_TONEMAPPER_VALUE",
        }
    }
}
impl From<i32> for TonemapperTypeBasic {
    fn from(v: i32) -> Self {
        assert!(v <= Self::MAX as i32);
        unsafe { std::mem::transmute(v) }
    }
}

impl std::fmt::Display for TonemapperTypeBasic {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub enum AntiAliasMethodBasic {
    None,
    Msaa4x,
    MAX,
}

impl Default for AntiAliasMethodBasic {
    fn default() -> Self {
        AntiAliasMethodBasic::Msaa4x
    }
}

impl AntiAliasMethodBasic {
    pub fn display_name(&self) -> &'static str {
        match self {
            AntiAliasMethodBasic::None => "None",
            AntiAliasMethodBasic::Msaa4x => "4x MSAA",
            AntiAliasMethodBasic::MAX => "AntiAliasMethodBasic MAX VALUE",
        }
    }
}

impl From<i32> for AntiAliasMethodBasic {
    fn from(v: i32) -> Self {
        assert!(v <= Self::MAX as i32);
        unsafe { std::mem::transmute(v) }
    }
}

#[derive(Clone)]
pub struct BasicPipelineRenderOptions {
    pub anti_alias_method: AntiAliasMethodBasic,
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
    pub tonemapper_type: TonemapperTypeBasic,
    pub enable_visibility_update: bool,
}

impl Default for BasicPipelineRenderOptions {
    fn default() -> Self {
        BasicPipelineRenderOptions {
            anti_alias_method: AntiAliasMethodBasic::Msaa4x,
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
            tonemapper_type: TonemapperTypeBasic::LogDerivative,
            enable_visibility_update: true,
        }
    }
}
