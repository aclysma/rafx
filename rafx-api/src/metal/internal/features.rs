use metal_rs::{MTLArgumentBuffersTier, MTLFeatureSet, MTLGPUFamily, MTLPixelFormat};

const GPU_FAMILIES_APPLE: [MTLGPUFamily; 6] = [
    MTLGPUFamily::Apple6,
    MTLGPUFamily::Apple5,
    MTLGPUFamily::Apple4,
    MTLGPUFamily::Apple3,
    MTLGPUFamily::Apple2,
    MTLGPUFamily::Apple1,
];

const GPU_FAMILIES_MAC: [MTLGPUFamily; 2] = [MTLGPUFamily::Mac2, MTLGPUFamily::Mac1];

const GPU_FAMILIES_COMMON: [MTLGPUFamily; 3] = [
    MTLGPUFamily::Common3,
    MTLGPUFamily::Common2,
    MTLGPUFamily::Common1,
];

const FEATURE_SETS_IOS: [MTLFeatureSet; 17] = [
    MTLFeatureSet::iOS_GPUFamily5_v1,
    MTLFeatureSet::iOS_GPUFamily4_v2,
    MTLFeatureSet::iOS_GPUFamily4_v1,
    MTLFeatureSet::iOS_GPUFamily3_v4,
    MTLFeatureSet::iOS_GPUFamily3_v3,
    MTLFeatureSet::iOS_GPUFamily3_v2,
    MTLFeatureSet::iOS_GPUFamily3_v1,
    MTLFeatureSet::iOS_GPUFamily2_v5,
    MTLFeatureSet::iOS_GPUFamily2_v4,
    MTLFeatureSet::iOS_GPUFamily2_v3,
    MTLFeatureSet::iOS_GPUFamily2_v2,
    MTLFeatureSet::iOS_GPUFamily2_v1,
    MTLFeatureSet::iOS_GPUFamily1_v5,
    MTLFeatureSet::iOS_GPUFamily1_v4,
    MTLFeatureSet::iOS_GPUFamily1_v3,
    MTLFeatureSet::iOS_GPUFamily1_v2,
    MTLFeatureSet::iOS_GPUFamily1_v1,
];

const FEATURE_SETS_TVOS: [MTLFeatureSet; 6] = [
    MTLFeatureSet::tvOS_GPUFamily2_v2,
    MTLFeatureSet::tvOS_GPUFamily2_v1,
    MTLFeatureSet::tvOS_GPUFamily1_v4,
    MTLFeatureSet::tvOS_GPUFamily1_v3,
    MTLFeatureSet::tvOS_GPUFamily1_v2,
    MTLFeatureSet::tvOS_GPUFamily1_v1,
];

const FEATURE_SETS_MACOS: [MTLFeatureSet; 5] = [
    MTLFeatureSet::macOS_GPUFamily2_v1,
    MTLFeatureSet::macOS_GPUFamily1_v4,
    MTLFeatureSet::macOS_GPUFamily1_v3,
    MTLFeatureSet::macOS_GPUFamily1_v2,
    MTLFeatureSet::macOS_GPUFamily1_v1,
];

fn find_supported_family(
    device: &metal_rs::DeviceRef,
    gpu_families: &[MTLGPUFamily],
) -> Option<MTLGPUFamily> {
    for &family in gpu_families {
        if device.supports_family(family) {
            return Some(family);
        }
    }

    return None;
}

fn find_supported_feature_set(
    device: &metal_rs::DeviceRef,
    feature_sets: &[MTLFeatureSet],
) -> Option<MTLFeatureSet> {
    for &feature_set in feature_sets {
        if device.supports_feature_set(feature_set) {
            return Some(feature_set);
        }
    }

    return None;
}

fn pixel_format_capabilities(
    feature_set: MTLFeatureSet,
    pixel_format: MTLPixelFormat,
) -> metal_rs::PixelFormatCapabilities {
    use metal_rs::PixelFormatCapabilities;
    match pixel_format {
        MTLPixelFormat::Invalid => PixelFormatCapabilities::empty(),
        MTLPixelFormat::A8Unorm => feature_set.a8_unorm_capabilities(),
        MTLPixelFormat::R8Unorm => feature_set.r8_unorm_capabilities(),
        MTLPixelFormat::R8Unorm_sRGB => feature_set.r8_unorm_srgb_capabilities(),
        MTLPixelFormat::R8Snorm => feature_set.r8_snorm_capabilities(),
        MTLPixelFormat::R8Uint => feature_set.r8_uint_capabilities(),
        MTLPixelFormat::R8Sint => feature_set.r8_sint_capabilities(),
        MTLPixelFormat::R16Unorm => feature_set.r16_unorm_capabilities(),
        MTLPixelFormat::R16Snorm => feature_set.r16_snorm_capabilities(),
        MTLPixelFormat::R16Uint => feature_set.r16_uint_capabilities(),
        MTLPixelFormat::R16Sint => feature_set.r16_sint_capabilities(),
        MTLPixelFormat::R16Float => feature_set.r16_float_capabilities(),
        MTLPixelFormat::RG8Unorm => feature_set.rg8_unorm_capabilities(),
        MTLPixelFormat::RG8Unorm_sRGB => feature_set.rg8_unorm_srgb_capabilities(),
        MTLPixelFormat::RG8Snorm => feature_set.rg8_snorm_capabilities(),
        MTLPixelFormat::RG8Uint => feature_set.rg8_uint_capabilities(),
        MTLPixelFormat::RG8Sint => feature_set.rg8_sint_capabilities(),
        MTLPixelFormat::B5G6R5Unorm => feature_set.b5_g6_r5_unorm_capabilities(),
        MTLPixelFormat::A1BGR5Unorm => feature_set.a1_bgr5_unorm_capabilities(),
        MTLPixelFormat::ABGR4Unorm => feature_set.abgr4_unorm_capabilities(),
        MTLPixelFormat::BGR5A1Unorm => feature_set.bgr5_a1_unorm_capabilities(),
        MTLPixelFormat::R32Uint => feature_set.r32_uint_capabilities(),
        MTLPixelFormat::R32Sint => feature_set.r32_sint_capabilities(),
        MTLPixelFormat::R32Float => feature_set.r32_float_capabilities(),
        MTLPixelFormat::RG16Unorm => feature_set.rg16_unorm_capabilities(),
        MTLPixelFormat::RG16Snorm => feature_set.rg16_snorm_capabilities(),
        MTLPixelFormat::RG16Uint => feature_set.rg16_uint_capabilities(),
        MTLPixelFormat::RG16Sint => feature_set.rg16_sint_capabilities(),
        MTLPixelFormat::RG16Float => feature_set.rg16_float_capabilities(),
        MTLPixelFormat::RGBA8Unorm => feature_set.rgba8_unorm_capabilities(),
        MTLPixelFormat::RGBA8Unorm_sRGB => feature_set.rgba8_unorm_srgb_capabilities(),
        MTLPixelFormat::RGBA8Snorm => feature_set.rgba8_snorm_capabilities(),
        MTLPixelFormat::RGBA8Uint => feature_set.rgba8_uint_capabilities(),
        MTLPixelFormat::RGBA8Sint => feature_set.rgba8_sint_capabilities(),
        MTLPixelFormat::BGRA8Unorm => feature_set.bgra8_unorm_capabilities(),
        MTLPixelFormat::BGRA8Unorm_sRGB => feature_set.bgra8_unorm_srgb_capabilities(),
        MTLPixelFormat::RGB10A2Unorm => feature_set.rgb10_a2_unorm_capabilities(),
        MTLPixelFormat::RGB10A2Uint => feature_set.rgb10_a2_uint_capabilities(),
        MTLPixelFormat::RG11B10Float => feature_set.rg11_b10_float_capabilities(),
        MTLPixelFormat::RGB9E5Float => feature_set.rgb9_e5_float_capabilities(),
        MTLPixelFormat::BGR10A2Unorm => feature_set.bgr10_a2_unorm_capabilities(),
        MTLPixelFormat::RG32Uint => feature_set.rg32_uint_capabilities(),
        MTLPixelFormat::RG32Sint => feature_set.rg32_sint_capabilities(),
        MTLPixelFormat::RG32Float => feature_set.rg32_float_capabilities(),
        MTLPixelFormat::RGBA16Unorm => feature_set.rgba16_unorm_capabilities(),
        MTLPixelFormat::RGBA16Snorm => feature_set.rgba16_snorm_capabilities(),
        MTLPixelFormat::RGBA16Uint => feature_set.rgba16_uint_capabilities(),
        MTLPixelFormat::RGBA16Sint => feature_set.rgba16_sint_capabilities(),
        MTLPixelFormat::RGBA16Float => feature_set.rgba16_float_capabilities(),
        MTLPixelFormat::RGBA32Uint => feature_set.rgba32_uint_capabilities(),
        MTLPixelFormat::RGBA32Sint => feature_set.rgba32_sint_capabilities(),
        MTLPixelFormat::RGBA32Float => feature_set.rgba32_float_capabilities(),
        MTLPixelFormat::BC1_RGBA => feature_set.bc_pixel_formats_capabilities(),
        MTLPixelFormat::BC1_RGBA_sRGB => feature_set.bc_pixel_formats_capabilities(),
        MTLPixelFormat::BC2_RGBA => feature_set.bc_pixel_formats_capabilities(),
        MTLPixelFormat::BC2_RGBA_sRGB => feature_set.bc_pixel_formats_capabilities(),
        MTLPixelFormat::BC3_RGBA => feature_set.bc_pixel_formats_capabilities(),
        MTLPixelFormat::BC3_RGBA_sRGB => feature_set.bc_pixel_formats_capabilities(),
        MTLPixelFormat::BC4_RUnorm => feature_set.bc_pixel_formats_capabilities(),
        MTLPixelFormat::BC4_RSnorm => feature_set.bc_pixel_formats_capabilities(),
        MTLPixelFormat::BC5_RGUnorm => feature_set.bc_pixel_formats_capabilities(),
        MTLPixelFormat::BC5_RGSnorm => feature_set.bc_pixel_formats_capabilities(),
        MTLPixelFormat::BC6H_RGBFloat => feature_set.bc_pixel_formats_capabilities(),
        MTLPixelFormat::BC6H_RGBUfloat => feature_set.bc_pixel_formats_capabilities(),
        MTLPixelFormat::BC7_RGBAUnorm => feature_set.bc_pixel_formats_capabilities(),
        MTLPixelFormat::BC7_RGBAUnorm_sRGB => feature_set.bc_pixel_formats_capabilities(),
        MTLPixelFormat::PVRTC_RGB_2BPP => feature_set.pvrtc_pixel_formats_capabilities(),
        MTLPixelFormat::PVRTC_RGB_2BPP_sRGB => feature_set.pvrtc_pixel_formats_capabilities(),
        MTLPixelFormat::PVRTC_RGB_4BPP => feature_set.pvrtc_pixel_formats_capabilities(),
        MTLPixelFormat::PVRTC_RGB_4BPP_sRGB => feature_set.pvrtc_pixel_formats_capabilities(),
        MTLPixelFormat::PVRTC_RGBA_2BPP => feature_set.pvrtc_pixel_formats_capabilities(),
        MTLPixelFormat::PVRTC_RGBA_2BPP_sRGB => feature_set.pvrtc_pixel_formats_capabilities(),
        MTLPixelFormat::PVRTC_RGBA_4BPP => feature_set.pvrtc_pixel_formats_capabilities(),
        MTLPixelFormat::PVRTC_RGBA_4BPP_sRGB => feature_set.pvrtc_pixel_formats_capabilities(),
        MTLPixelFormat::EAC_R11Unorm => feature_set.eac_etc_pixel_formats_capabilities(),
        MTLPixelFormat::EAC_R11Snorm => feature_set.eac_etc_pixel_formats_capabilities(),
        MTLPixelFormat::EAC_RG11Unorm => feature_set.eac_etc_pixel_formats_capabilities(),
        MTLPixelFormat::EAC_RG11Snorm => feature_set.eac_etc_pixel_formats_capabilities(),
        MTLPixelFormat::EAC_RGBA8 => feature_set.eac_etc_pixel_formats_capabilities(),
        MTLPixelFormat::EAC_RGBA8_sRGB => feature_set.eac_etc_pixel_formats_capabilities(),
        MTLPixelFormat::ETC2_RGB8 => feature_set.eac_etc_pixel_formats_capabilities(),
        MTLPixelFormat::ETC2_RGB8_sRGB => feature_set.eac_etc_pixel_formats_capabilities(),
        MTLPixelFormat::ETC2_RGB8A1 => feature_set.eac_etc_pixel_formats_capabilities(),
        MTLPixelFormat::ETC2_RGB8A1_sRGB => feature_set.eac_etc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_4x4_sRGB => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_5x4_sRGB => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_5x5_sRGB => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_6x5_sRGB => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_6x6_sRGB => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_8x5_sRGB => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_8x6_sRGB => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_8x8_sRGB => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_10x5_sRGB => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_10x6_sRGB => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_10x8_sRGB => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_10x10_sRGB => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_12x10_sRGB => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_12x12_sRGB => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_4x4_LDR => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_5x4_LDR => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_5x5_LDR => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_6x5_LDR => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_6x6_LDR => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_8x5_LDR => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_8x6_LDR => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_8x8_LDR => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_10x5_LDR => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_10x6_LDR => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_10x8_LDR => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_10x10_LDR => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_12x10_LDR => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::ASTC_12x12_LDR => feature_set.astc_pixel_formats_capabilities(),
        MTLPixelFormat::GBGR422 => feature_set.gbgr422_capabilities(),
        MTLPixelFormat::BGRG422 => feature_set.bgrg422_capabilities(),
        MTLPixelFormat::Depth16Unorm => feature_set.depth16_unorm_capabilities(),
        MTLPixelFormat::Depth32Float => feature_set.depth32_float_capabilities(),
        MTLPixelFormat::Stencil8 => feature_set.stencil8_capabilities(),
        MTLPixelFormat::Depth24Unorm_Stencil8 => feature_set.depth24_unorm_stencil8_capabilities(),
        MTLPixelFormat::Depth32Float_Stencil8 => feature_set.depth32_float_stencil8_capabilities(),
        MTLPixelFormat::X32_Stencil8 => feature_set.x32_stencil8_capabilities(),
        MTLPixelFormat::X24_Stencil8 => feature_set.x24_stencil8_capabilities(),
        MTLPixelFormat::BGRA10_XR => feature_set.bgra10_xr_capabilities(),
        MTLPixelFormat::BGRA10_XR_SRGB => feature_set.bgra10_xr_srgb_capabilities(),
        MTLPixelFormat::BGR10_XR => feature_set.bgr10_xr_capabilities(),
        MTLPixelFormat::BGR10_XR_SRGB => feature_set.bgr10_xr_srgb_capabilities(),
    }
}

#[derive(Debug)]
pub struct MetalFeatures {
    pub device_name: String,
    pub unified_memory: bool,
    pub is_low_power: bool,
    pub argument_buffers_tier: MTLArgumentBuffersTier,
    pub gpu_family_apple: Option<MTLGPUFamily>,
    pub gpu_family_mac: Option<MTLGPUFamily>,
    pub gpu_family_common: Option<MTLGPUFamily>,
    pub feature_set_ios: Option<MTLFeatureSet>,
    pub feature_set_macos: Option<MTLFeatureSet>,
    pub feature_set_tvos: Option<MTLFeatureSet>,
    pub supports_argument_buffers: bool,
    pub supports_array_of_samplers: bool,
    pub supports_array_of_textures: bool,
    pub supports_base_vertex_instance_drawing: bool,
    pub supports_combined_msaa_store_and_resolve_action: bool,
    pub supports_cube_map_texture_arrays: bool,
    pub supports_resource_heaps: bool,
}

impl MetalFeatures {
    pub fn from_device(device: &metal_rs::DeviceRef) -> Self {
        let device_name = device.name().to_string();
        let unified_memory = device.has_unified_memory();
        let is_low_power = device.is_low_power();
        let argument_buffers_tier = device.argument_buffers_support();
        let gpu_family_apple = find_supported_family(device, &GPU_FAMILIES_APPLE);
        let gpu_family_mac = find_supported_family(device, &GPU_FAMILIES_MAC);
        let gpu_family_common = find_supported_family(device, &GPU_FAMILIES_COMMON);
        let feature_set_ios = find_supported_feature_set(device, &FEATURE_SETS_IOS);
        let feature_set_macos = find_supported_feature_set(device, &FEATURE_SETS_MACOS);
        let feature_set_tvos = find_supported_feature_set(device, &FEATURE_SETS_TVOS);

        let mut supports_argument_buffers = false;
        let mut supports_array_of_samplers = false;
        let mut supports_array_of_textures = false;
        let mut supports_base_vertex_instance_drawing = false;
        let mut supports_combined_msaa_store_and_resolve_action = false;
        let mut supports_cube_map_texture_arrays = false;
        let mut supports_resource_heaps = false;

        if let Some(feature_set_ios) = feature_set_ios {
            supports_argument_buffers = feature_set_ios.supports_argument_buffers();
            supports_array_of_samplers = feature_set_ios.supports_array_of_samplers();
            supports_array_of_textures = feature_set_ios.supports_array_of_textures();
            supports_base_vertex_instance_drawing =
                feature_set_ios.supports_base_vertex_instance_drawing();
            supports_combined_msaa_store_and_resolve_action =
                feature_set_ios.supports_combined_msaa_store_and_resolve_action();
            supports_cube_map_texture_arrays = feature_set_ios.supports_cube_map_texture_arrays();
            supports_resource_heaps = feature_set_ios.supports_resource_heaps();
        }

        if let Some(feature_set_macos) = feature_set_macos {
            supports_argument_buffers = feature_set_macos.supports_argument_buffers();
            supports_array_of_samplers = feature_set_macos.supports_array_of_samplers();
            supports_array_of_textures = feature_set_macos.supports_array_of_textures();
            supports_base_vertex_instance_drawing =
                feature_set_macos.supports_base_vertex_instance_drawing();
            supports_combined_msaa_store_and_resolve_action =
                feature_set_macos.supports_combined_msaa_store_and_resolve_action();
            supports_cube_map_texture_arrays = feature_set_macos.supports_cube_map_texture_arrays();
            supports_resource_heaps = feature_set_macos.supports_resource_heaps();
        }

        if let Some(feature_set_tvos) = feature_set_tvos {
            supports_argument_buffers = feature_set_tvos.supports_argument_buffers();
            supports_array_of_samplers = feature_set_tvos.supports_array_of_samplers();
            supports_array_of_textures = feature_set_tvos.supports_array_of_textures();
            supports_base_vertex_instance_drawing =
                feature_set_tvos.supports_base_vertex_instance_drawing();
            supports_combined_msaa_store_and_resolve_action =
                feature_set_tvos.supports_combined_msaa_store_and_resolve_action();
            supports_cube_map_texture_arrays = feature_set_tvos.supports_cube_map_texture_arrays();
            supports_resource_heaps = feature_set_tvos.supports_resource_heaps();
        }

        MetalFeatures {
            device_name,
            unified_memory,
            is_low_power,
            argument_buffers_tier,
            gpu_family_apple,
            gpu_family_mac,
            gpu_family_common,
            feature_set_ios,
            feature_set_macos,
            feature_set_tvos,
            supports_argument_buffers,
            supports_array_of_samplers,
            supports_array_of_textures,
            supports_base_vertex_instance_drawing,
            supports_combined_msaa_store_and_resolve_action,
            supports_cube_map_texture_arrays,
            supports_resource_heaps,
        }
    }

    pub fn pixel_format_capabilities(
        &self,
        pixel_format: MTLPixelFormat,
    ) -> metal_rs::PixelFormatCapabilities {
        let mut capabilities = metal_rs::PixelFormatCapabilities::empty();
        if let Some(feature_set_ios) = self.feature_set_ios {
            capabilities |= pixel_format_capabilities(feature_set_ios, pixel_format);
        }
        if let Some(feature_set_macos) = self.feature_set_macos {
            capabilities |= pixel_format_capabilities(feature_set_macos, pixel_format);
        }
        if let Some(feature_set_tvos) = self.feature_set_tvos {
            capabilities |= pixel_format_capabilities(feature_set_tvos, pixel_format);
        }
        capabilities
    }
}
