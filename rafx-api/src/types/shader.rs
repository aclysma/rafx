use fnv::FnvHasher;
use std::hash::{Hash, Hasher};

use crate::{RafxApiType, RafxReflectedEntryPoint};
#[cfg(feature = "serde-support")]
use serde::{Deserialize, Serialize};

/// GL ES 2.0-specific shader package. Can be used to create a RafxShaderModuleDef, which in turn is
/// used to initialize a shader module GPU object
///
/// It is a struct rather than an enum because these are not mutually exclusive
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum RafxShaderPackageGles2 {
    /// Raw uncompiled OpenGL ES 2.0 source code. Will be compiled at runtime.
    Src(String),
}

/// GL ES 3.0-specific shader package. Can be used to create a RafxShaderModuleDef, which in turn is
/// used to initialize a shader module GPU object
///
/// It is a struct rather than an enum because these are not mutually exclusive
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum RafxShaderPackageGles3 {
    /// Raw uncompiled OpenGL ES 3.0 source code. Will be compiled at runtime.
    Src(String),
}

/// Metal-specific shader package. Can be used to create a RafxShaderModuleDef, which in turn is
/// used to initialize a shader module GPU object
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum RafxShaderPackageDx12 {
    /// Raw uncompiled source code. Will be compiled at runtime.
    Src(String),
    // DXIL compiles from dxc
    //Dxil(Vec<u8>)
}

/// Metal-specific shader package. Can be used to create a RafxShaderModuleDef, which in turn is
/// used to initialize a shader module GPU object
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum RafxShaderPackageMetal {
    /// Raw uncompiled source code. Will be compiled at runtime.
    Src(String),
    /// Pre-built binary "metallib" file loaded into memory
    #[cfg_attr(feature = "serde-support", serde(with = "serde_bytes"))]
    LibBytes(Vec<u8>),
}

/// Vulkan-specific shader package. Can be used to create a RafxShaderModuleDef, which in turn is
/// used to initialize a shader module GPU object
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub enum RafxShaderPackageVulkan {
    /// Raw SPV bytes, no alignment or endianness requirements.
    #[cfg_attr(feature = "serde-support", serde(with = "serde_bytes"))]
    SpvBytes(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
#[doc(hidden)]
pub enum RafxShaderPackageEmpty {
    Empty,
}

/// Owns data necessary to create a shader module in (optionally) multiple APIs.
///
/// This struct can be serialized/deserialized and is intended to allow asset pipeline to store
/// a shader module to be created at runtime. The package can optionally include data for multiple
/// APIs allowing a single file to be used with whatever API is found at runtime.
///
/// Optionally, reflection data can be packaged along with shaders. Shaders may have
/// platform-specific changes that produce different reflection data, so reflection is stored
/// per-api
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct RafxShaderPackage {
    pub gles2: Option<RafxShaderPackageGles2>,
    pub gles3: Option<RafxShaderPackageGles3>,
    pub dx12: Option<RafxShaderPackageDx12>,
    pub metal: Option<RafxShaderPackageMetal>,
    pub vk: Option<RafxShaderPackageVulkan>,

    pub vk_reflection: Option<Vec<RafxReflectedEntryPoint>>,
    pub dx12_reflection: Option<Vec<RafxReflectedEntryPoint>>,
    pub metal_reflection: Option<Vec<RafxReflectedEntryPoint>>,
    pub gles2_reflection: Option<Vec<RafxReflectedEntryPoint>>,
    pub gles3_reflection: Option<Vec<RafxReflectedEntryPoint>>,

    pub debug_name: Option<String>,
}

/// Provides a stable has for contents of a shader package
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct RafxShaderPackageHash(u64);
impl RafxShaderPackageHash {
    pub fn new(shader_package: &RafxShaderPackage) -> Self {
        let mut hasher = FnvHasher::default();
        shader_package.hash(&mut hasher);
        let hash = hasher.finish();
        RafxShaderPackageHash(hash)
    }
}

/// A shader package and its hash. This allows storing the package with a pre-generated hash to
/// file. The shader package is immutable to ensure the hash is never stale.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde-support", derive(Serialize, Deserialize))]
pub struct RafxHashedShaderPackage {
    shader_package_hash: RafxShaderPackageHash,
    shader_package: RafxShaderPackage,
}

impl RafxHashedShaderPackage {
    pub fn new(shader_package: RafxShaderPackage) -> Self {
        let shader_package_hash = RafxShaderPackageHash::new(&shader_package);
        RafxHashedShaderPackage {
            shader_package_hash,
            shader_package,
        }
    }

    pub fn shader_package_hash(&self) -> RafxShaderPackageHash {
        self.shader_package_hash
    }

    pub fn shader_package(&self) -> &RafxShaderPackage {
        &self.shader_package
    }
}

impl RafxShaderPackage {
    pub fn reflection(
        &self,
        api_type: RafxApiType,
    ) -> Option<&Vec<RafxReflectedEntryPoint>> {
        match api_type {
            RafxApiType::Vk => self.vk_reflection.as_ref(),
            RafxApiType::Dx12 => self.dx12_reflection.as_ref(),
            RafxApiType::Metal => self.metal_reflection.as_ref(),
            RafxApiType::Gles2 => self.gles2_reflection.as_ref(),
            RafxApiType::Gles3 => self.gles3_reflection.as_ref(),
            RafxApiType::Empty => None,
        }
    }

    pub fn find_entry_point(
        &self,
        api_type: RafxApiType,
        entry_point_name: &str,
    ) -> Option<&RafxReflectedEntryPoint> {
        self.reflection(api_type)
            .map(|x| {
                x.iter()
                    .find(|&x| x.rafx_api_reflection.entry_point_name == entry_point_name)
            })
            .flatten()
    }

    /// Create a shader module def for use with a GL RafxDevice. Returns none if the package does
    /// not contain data necessary for GL ES 2.0
    #[cfg(feature = "rafx-gles2")]
    pub fn gles2_module_def(&self) -> Option<RafxShaderModuleDefGles2> {
        if let Some(gl) = self.gles2.as_ref() {
            Some(match gl {
                RafxShaderPackageGles2::Src(src) => RafxShaderModuleDefGles2::GlSrc(src),
            })
        } else {
            None
        }
    }

    /// Create a shader module def for use with a GL RafxDevice. Returns none if the package does
    /// not contain data necessary for GL ES 2.0
    #[cfg(feature = "rafx-gles3")]
    pub fn gles3_module_def(&self) -> Option<RafxShaderModuleDefGles3> {
        if let Some(gl) = self.gles3.as_ref() {
            Some(match gl {
                RafxShaderPackageGles3::Src(src) => RafxShaderModuleDefGles3::GlSrc(src),
            })
        } else {
            None
        }
    }

    /// Create a shader module def for use with a dx12 RafxDevice. Returns none if the package does
    /// not contain data necessary for metal
    #[cfg(feature = "rafx-dx12")]
    pub fn dx12_module_def(&self) -> Option<RafxShaderModuleDefDx12> {
        if let Some(dx12) = self.dx12.as_ref() {
            Some(match dx12 {
                RafxShaderPackageDx12::Src(src) => RafxShaderModuleDefDx12::HlslSrc(src),
                //RafxShaderPackageDx12::Dxil(dxil) => RafxShaderModuleDefDx12::Dxil(dxil.as_slice()),
            })
        } else {
            None
        }
    }

    /// Create a shader module def for use with a metal RafxDevice. Returns none if the package does
    /// not contain data necessary for metal
    #[cfg(feature = "rafx-metal")]
    pub fn metal_module_def(&self) -> Option<RafxShaderModuleDefMetal> {
        if let Some(metal) = self.metal.as_ref() {
            Some(match metal {
                RafxShaderPackageMetal::Src(src) => RafxShaderModuleDefMetal::MetalSrc(src),
                RafxShaderPackageMetal::LibBytes(lib) => {
                    RafxShaderModuleDefMetal::MetalLibBytes(lib)
                }
            })
        } else {
            None
        }
    }

    /// Create a shader module def for use with a vulkan RafxDevice. Returns none if the package
    /// does not contain data necessary for vulkan
    #[cfg(feature = "rafx-vulkan")]
    pub fn vulkan_module_def(&self) -> Option<RafxShaderModuleDefVulkan> {
        if let Some(vk) = self.vk.as_ref() {
            Some(match vk {
                RafxShaderPackageVulkan::SpvBytes(bytes) => {
                    RafxShaderModuleDefVulkan::VkSpvBytes(bytes)
                }
            })
        } else {
            None
        }
    }

    /// Create a shader module def for use with a vulkan RafxDevice. Returns none if the package
    /// does not contain data necessary for vulkan
    #[cfg(any(
        feature = "rafx-empty",
        not(any(
            feature = "rafx-dx12",
            feature = "rafx-metal",
            feature = "rafx-vulkan",
            feature = "rafx-gles2",
            feature = "rafx-gles3"
        ))
    ))]
    #[doc(hidden)]
    pub fn empty_module_def(&self) -> Option<RafxShaderModuleDefEmpty> {
        Some(RafxShaderModuleDefEmpty::Empty(Default::default()))
    }

    pub fn module_def(&self) -> RafxShaderModuleDef {
        RafxShaderModuleDef {
            #[cfg(feature = "rafx-gles2")]
            gles2: self.gles2_module_def(),
            #[cfg(feature = "rafx-gles3")]
            gles3: self.gles3_module_def(),
            #[cfg(feature = "rafx-dx12")]
            dx12: self.dx12_module_def(),
            #[cfg(feature = "rafx-metal")]
            metal: self.metal_module_def(),
            #[cfg(feature = "rafx-vulkan")]
            vk: self.vulkan_module_def(),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            #[doc(hidden)]
            empty: self.empty_module_def(),
        }
    }
}

/// Used to create a RafxShaderModule
///
/// This enum may be populated manually or created from a RafxShaderPackage.
#[derive(Copy, Clone, Hash)]
#[cfg(feature = "rafx-gles2")]
pub enum RafxShaderModuleDefGles2<'a> {
    /// GL source code
    GlSrc(&'a str),
}

/// Used to create a RafxShaderModule
///
/// This enum may be populated manually or created from a RafxShaderPackage.
#[derive(Copy, Clone, Hash)]
#[cfg(feature = "rafx-gles3")]
pub enum RafxShaderModuleDefGles3<'a> {
    /// GL source code
    GlSrc(&'a str),
}

/// Used to create a RafxShaderModule
///
/// This enum may be populated manually or created from a RafxShaderPackage.
#[derive(Copy, Clone, Hash)]
#[cfg(feature = "rafx-dx12")]
pub enum RafxShaderModuleDefDx12<'a> {
    /// HLSL source code
    HlslSrc(&'a str),
    // Compiled DXIL from dxc
    //Dxil(&'a [u8]),
}

/// Used to create a RafxShaderModule
///
/// This enum may be populated manually or created from a RafxShaderPackage.
#[derive(Copy, Clone, Hash)]
#[cfg(feature = "rafx-metal")]
pub enum RafxShaderModuleDefMetal<'a> {
    /// Metal source code
    MetalSrc(&'a str),
    /// Pre-compiled library loaded as bytes
    MetalLibBytes(&'a [u8]),
}

/// Used to create a RafxShaderModule
///
/// This enum may be populated manually or created from a RafxShaderPackage.
#[derive(Copy, Clone, Hash)]
#[cfg(feature = "rafx-vulkan")]
pub enum RafxShaderModuleDefVulkan<'a> {
    /// Raw SPV bytes, no alignment or endianness requirements.
    VkSpvBytes(&'a [u8]),
    /// Prepared SPV that's aligned and correct endian. No validation.
    VkSpvPrepared(&'a [u32]),
}

#[cfg(any(
    feature = "rafx-empty",
    not(any(
        feature = "rafx-dx12",
        feature = "rafx-metal",
        feature = "rafx-vulkan",
        feature = "rafx-gles2",
        feature = "rafx-gles3"
    ))
))]
#[derive(Copy, Clone, Hash)]
#[doc(hidden)]
pub enum RafxShaderModuleDefEmpty<'a> {
    Empty(std::marker::PhantomData<&'a u32>),
}

/// Used to create a RafxShaderModule
///
/// This enum may be populated manually or created from a RafxShaderPackage.
#[derive(Copy, Clone, Hash, Default)]
pub struct RafxShaderModuleDef<'a> {
    #[cfg(feature = "rafx-gles2")]
    pub gles2: Option<RafxShaderModuleDefGles2<'a>>,
    #[cfg(feature = "rafx-gles3")]
    pub gles3: Option<RafxShaderModuleDefGles3<'a>>,
    #[cfg(feature = "rafx-dx12")]
    pub dx12: Option<RafxShaderModuleDefDx12<'a>>,
    #[cfg(feature = "rafx-metal")]
    pub metal: Option<RafxShaderModuleDefMetal<'a>>,
    #[cfg(feature = "rafx-vulkan")]
    pub vk: Option<RafxShaderModuleDefVulkan<'a>>,
    #[cfg(any(
        feature = "rafx-empty",
        not(any(
            feature = "rafx-dx12",
            feature = "rafx-metal",
            feature = "rafx-vulkan",
            feature = "rafx-gles2",
            feature = "rafx-gles3"
        ))
    ))]
    #[doc(hidden)]
    pub empty: Option<RafxShaderModuleDefEmpty<'a>>,
}
