// This file generated automatically by hydrate-codegen. Do not make manual edits. Use include!() to place these types in the intended location.
#[derive(Default)]
pub struct GpuCompressedImageAssetRecord(PropertyPath);

impl Field for GpuCompressedImageAssetRecord {
    fn new(property_path: PropertyPath) -> Self {
        GpuCompressedImageAssetRecord(property_path)
    }
}

impl Record for GpuCompressedImageAssetRecord {
    fn schema_name() -> &'static str {
        "GpuCompressedImageAsset"
    }
}

impl GpuCompressedImageAssetRecord {
}
#[derive(Default)]
pub struct GpuCompressedImageImportedDataRecord(PropertyPath);

impl Field for GpuCompressedImageImportedDataRecord {
    fn new(property_path: PropertyPath) -> Self {
        GpuCompressedImageImportedDataRecord(property_path)
    }
}

impl Record for GpuCompressedImageImportedDataRecord {
    fn schema_name() -> &'static str {
        "GpuCompressedImageImportedData"
    }
}

impl GpuCompressedImageImportedDataRecord {
    pub fn data_layers(&self) -> DynamicArrayField::<GpuImageSubresourceLayerRecord> {
        DynamicArrayField::<GpuImageSubresourceLayerRecord>::new(self.0.push("data_layers"))
    }

    pub fn data_single_buffer(&self) -> BytesField {
        BytesField::new(self.0.push("data_single_buffer"))
    }

    pub fn format(&self) -> EnumField::<GpuImageAssetDataFormatEnum> {
        EnumField::<GpuImageAssetDataFormatEnum>::new(self.0.push("format"))
    }

    pub fn height(&self) -> U32Field {
        U32Field::new(self.0.push("height"))
    }

    pub fn is_cube_texture(&self) -> BooleanField {
        BooleanField::new(self.0.push("is_cube_texture"))
    }

    pub fn width(&self) -> U32Field {
        U32Field::new(self.0.push("width"))
    }
}
#[derive(Default)]
pub struct GpuImageAssetRecord(PropertyPath);

impl Field for GpuImageAssetRecord {
    fn new(property_path: PropertyPath) -> Self {
        GpuImageAssetRecord(property_path)
    }
}

impl Record for GpuImageAssetRecord {
    fn schema_name() -> &'static str {
        "GpuImageAsset"
    }
}

impl GpuImageAssetRecord {
    pub fn basis_compression(&self) -> BooleanField {
        BooleanField::new(self.0.push("basis_compression"))
    }

    pub fn basis_compression_settings(&self) -> GpuImageBasisCompressionSettingsRecord {
        GpuImageBasisCompressionSettingsRecord::new(self.0.push("basis_compression_settings"))
    }

    pub fn color_space(&self) -> EnumField::<GpuImageColorSpaceEnum> {
        EnumField::<GpuImageColorSpaceEnum>::new(self.0.push("color_space"))
    }

    pub fn mip_generation(&self) -> EnumField::<GpuImageMipGenerationEnum> {
        EnumField::<GpuImageMipGenerationEnum>::new(self.0.push("mip_generation"))
    }
}
#[derive(Copy, Clone)]
pub enum GpuImageAssetDataFormatEnum {
    RGBA32_Linear,
    RGBA32_Srgb,
    Basis_Linear,
    Basis_Srgb,
    BC1_UNorm_Linear,
    BC1_UNorm_Srgb,
    BC2_UNorm_Linear,
    BC2_UNorm_Srgb,
    BC3_UNorm_Linear,
    BC3_UNorm_Srgb,
    BC4_UNorm,
    BC4_SNorm,
    BC5_UNorm,
    BC5_SNorm,
    BC6H_UFloat,
    BC6H_SFloat,
    BC7_Unorm_Linear,
    BC7_Unorm_Srgb,
}

impl Enum for GpuImageAssetDataFormatEnum {
    fn to_symbol_name(&self) -> &'static str {
        match self {
            GpuImageAssetDataFormatEnum::RGBA32_Linear => "RGBA32_Linear",
            GpuImageAssetDataFormatEnum::RGBA32_Srgb => "RGBA32_Srgb",
            GpuImageAssetDataFormatEnum::Basis_Linear => "Basis_Linear",
            GpuImageAssetDataFormatEnum::Basis_Srgb => "Basis_Srgb",
            GpuImageAssetDataFormatEnum::BC1_UNorm_Linear => "BC1_UNorm_Linear",
            GpuImageAssetDataFormatEnum::BC1_UNorm_Srgb => "BC1_UNorm_Srgb",
            GpuImageAssetDataFormatEnum::BC2_UNorm_Linear => "BC2_UNorm_Linear",
            GpuImageAssetDataFormatEnum::BC2_UNorm_Srgb => "BC2_UNorm_Srgb",
            GpuImageAssetDataFormatEnum::BC3_UNorm_Linear => "BC3_UNorm_Linear",
            GpuImageAssetDataFormatEnum::BC3_UNorm_Srgb => "BC3_UNorm_Srgb",
            GpuImageAssetDataFormatEnum::BC4_UNorm => "BC4_UNorm",
            GpuImageAssetDataFormatEnum::BC4_SNorm => "BC4_SNorm",
            GpuImageAssetDataFormatEnum::BC5_UNorm => "BC5_UNorm",
            GpuImageAssetDataFormatEnum::BC5_SNorm => "BC5_SNorm",
            GpuImageAssetDataFormatEnum::BC6H_UFloat => "BC6H_UFloat",
            GpuImageAssetDataFormatEnum::BC6H_SFloat => "BC6H_SFloat",
            GpuImageAssetDataFormatEnum::BC7_Unorm_Linear => "BC7_Unorm_Linear",
            GpuImageAssetDataFormatEnum::BC7_Unorm_Srgb => "BC7_Unorm_Srgb",
        }
    }

    fn from_symbol_name(str: &str) -> Option<GpuImageAssetDataFormatEnum> {
        match str {
            "RGBA32_Linear" => Some(GpuImageAssetDataFormatEnum::RGBA32_Linear),
            "RGBA32_Srgb" => Some(GpuImageAssetDataFormatEnum::RGBA32_Srgb),
            "Basis_Linear" => Some(GpuImageAssetDataFormatEnum::Basis_Linear),
            "Basis_Srgb" => Some(GpuImageAssetDataFormatEnum::Basis_Srgb),
            "BC1_UNorm_Linear" => Some(GpuImageAssetDataFormatEnum::BC1_UNorm_Linear),
            "BC1_UNorm_Srgb" => Some(GpuImageAssetDataFormatEnum::BC1_UNorm_Srgb),
            "BC2_UNorm_Linear" => Some(GpuImageAssetDataFormatEnum::BC2_UNorm_Linear),
            "BC2_UNorm_Srgb" => Some(GpuImageAssetDataFormatEnum::BC2_UNorm_Srgb),
            "BC3_UNorm_Linear" => Some(GpuImageAssetDataFormatEnum::BC3_UNorm_Linear),
            "BC3_UNorm_Srgb" => Some(GpuImageAssetDataFormatEnum::BC3_UNorm_Srgb),
            "BC4_UNorm" => Some(GpuImageAssetDataFormatEnum::BC4_UNorm),
            "BC4_SNorm" => Some(GpuImageAssetDataFormatEnum::BC4_SNorm),
            "BC5_UNorm" => Some(GpuImageAssetDataFormatEnum::BC5_UNorm),
            "BC5_SNorm" => Some(GpuImageAssetDataFormatEnum::BC5_SNorm),
            "BC6H_UFloat" => Some(GpuImageAssetDataFormatEnum::BC6H_UFloat),
            "BC6H_SFloat" => Some(GpuImageAssetDataFormatEnum::BC6H_SFloat),
            "BC7_Unorm_Linear" => Some(GpuImageAssetDataFormatEnum::BC7_Unorm_Linear),
            "BC7_Unorm_Srgb" => Some(GpuImageAssetDataFormatEnum::BC7_Unorm_Srgb),
            _ => None,
        }
    }
}

impl GpuImageAssetDataFormatEnum {
    pub fn schema_name() -> &'static str {
        "GpuImageAssetDataFormat"
    }
}
#[derive(Default)]
pub struct GpuImageBasisCompressionSettingsRecord(PropertyPath);

impl Field for GpuImageBasisCompressionSettingsRecord {
    fn new(property_path: PropertyPath) -> Self {
        GpuImageBasisCompressionSettingsRecord(property_path)
    }
}

impl Record for GpuImageBasisCompressionSettingsRecord {
    fn schema_name() -> &'static str {
        "GpuImageBasisCompressionSettings"
    }
}

impl GpuImageBasisCompressionSettingsRecord {
    pub fn compression_type(&self) -> EnumField::<GpuImageBasisCompressionTypeEnum> {
        EnumField::<GpuImageBasisCompressionTypeEnum>::new(self.0.push("compression_type"))
    }

    pub fn quality(&self) -> U32Field {
        U32Field::new(self.0.push("quality"))
    }
}
#[derive(Copy, Clone)]
pub enum GpuImageBasisCompressionTypeEnum {
    Etc1S,
    Uastc,
}

impl Enum for GpuImageBasisCompressionTypeEnum {
    fn to_symbol_name(&self) -> &'static str {
        match self {
            GpuImageBasisCompressionTypeEnum::Etc1S => "Etc1S",
            GpuImageBasisCompressionTypeEnum::Uastc => "Uastc",
        }
    }

    fn from_symbol_name(str: &str) -> Option<GpuImageBasisCompressionTypeEnum> {
        match str {
            "Etc1S" => Some(GpuImageBasisCompressionTypeEnum::Etc1S),
            "Uastc" => Some(GpuImageBasisCompressionTypeEnum::Uastc),
            _ => None,
        }
    }
}

impl GpuImageBasisCompressionTypeEnum {
    pub fn schema_name() -> &'static str {
        "GpuImageBasisCompressionType"
    }
}
#[derive(Copy, Clone)]
pub enum GpuImageColorSpaceEnum {
    Srgb,
    Linear,
}

impl Enum for GpuImageColorSpaceEnum {
    fn to_symbol_name(&self) -> &'static str {
        match self {
            GpuImageColorSpaceEnum::Srgb => "Srgb",
            GpuImageColorSpaceEnum::Linear => "Linear",
        }
    }

    fn from_symbol_name(str: &str) -> Option<GpuImageColorSpaceEnum> {
        match str {
            "Srgb" => Some(GpuImageColorSpaceEnum::Srgb),
            "Linear" => Some(GpuImageColorSpaceEnum::Linear),
            _ => None,
        }
    }
}

impl GpuImageColorSpaceEnum {
    pub fn schema_name() -> &'static str {
        "GpuImageColorSpace"
    }
}
#[derive(Default)]
pub struct GpuImageImportedDataRecord(PropertyPath);

impl Field for GpuImageImportedDataRecord {
    fn new(property_path: PropertyPath) -> Self {
        GpuImageImportedDataRecord(property_path)
    }
}

impl Record for GpuImageImportedDataRecord {
    fn schema_name() -> &'static str {
        "GpuImageImportedData"
    }
}

impl GpuImageImportedDataRecord {
    pub fn height(&self) -> U32Field {
        U32Field::new(self.0.push("height"))
    }

    pub fn image_bytes(&self) -> BytesField {
        BytesField::new(self.0.push("image_bytes"))
    }

    pub fn width(&self) -> U32Field {
        U32Field::new(self.0.push("width"))
    }
}
#[derive(Copy, Clone)]
pub enum GpuImageMipGenerationEnum {
    NoMips,
    Precomputed,
    Runtime,
}

impl Enum for GpuImageMipGenerationEnum {
    fn to_symbol_name(&self) -> &'static str {
        match self {
            GpuImageMipGenerationEnum::NoMips => "NoMips",
            GpuImageMipGenerationEnum::Precomputed => "Precomputed",
            GpuImageMipGenerationEnum::Runtime => "Runtime",
        }
    }

    fn from_symbol_name(str: &str) -> Option<GpuImageMipGenerationEnum> {
        match str {
            "NoMips" => Some(GpuImageMipGenerationEnum::NoMips),
            "Precomputed" => Some(GpuImageMipGenerationEnum::Precomputed),
            "Runtime" => Some(GpuImageMipGenerationEnum::Runtime),
            _ => None,
        }
    }
}

impl GpuImageMipGenerationEnum {
    pub fn schema_name() -> &'static str {
        "GpuImageMipGeneration"
    }
}
#[derive(Default)]
pub struct GpuImageSubresourceLayerRecord(PropertyPath);

impl Field for GpuImageSubresourceLayerRecord {
    fn new(property_path: PropertyPath) -> Self {
        GpuImageSubresourceLayerRecord(property_path)
    }
}

impl Record for GpuImageSubresourceLayerRecord {
    fn schema_name() -> &'static str {
        "GpuImageSubresourceLayer"
    }
}

impl GpuImageSubresourceLayerRecord {
    pub fn mip_levels(&self) -> DynamicArrayField::<GpuImageSubresourceMipLevelRecord> {
        DynamicArrayField::<GpuImageSubresourceMipLevelRecord>::new(self.0.push("mip_levels"))
    }
}
#[derive(Default)]
pub struct GpuImageSubresourceMipLevelRecord(PropertyPath);

impl Field for GpuImageSubresourceMipLevelRecord {
    fn new(property_path: PropertyPath) -> Self {
        GpuImageSubresourceMipLevelRecord(property_path)
    }
}

impl Record for GpuImageSubresourceMipLevelRecord {
    fn schema_name() -> &'static str {
        "GpuImageSubresourceMipLevel"
    }
}

impl GpuImageSubresourceMipLevelRecord {
    pub fn bytes(&self) -> BytesField {
        BytesField::new(self.0.push("bytes"))
    }

    pub fn height(&self) -> U32Field {
        U32Field::new(self.0.push("height"))
    }

    pub fn width(&self) -> U32Field {
        U32Field::new(self.0.push("width"))
    }
}
#[derive(Default)]
pub struct Vec3Record(PropertyPath);

impl Field for Vec3Record {
    fn new(property_path: PropertyPath) -> Self {
        Vec3Record(property_path)
    }
}

impl Record for Vec3Record {
    fn schema_name() -> &'static str {
        "Vec3"
    }
}

impl Vec3Record {
    pub fn x(&self) -> F32Field {
        F32Field::new(self.0.push("x"))
    }

    pub fn y(&self) -> F32Field {
        F32Field::new(self.0.push("y"))
    }

    pub fn z(&self) -> F32Field {
        F32Field::new(self.0.push("z"))
    }
}
#[derive(Default)]
pub struct Vec4Record(PropertyPath);

impl Field for Vec4Record {
    fn new(property_path: PropertyPath) -> Self {
        Vec4Record(property_path)
    }
}

impl Record for Vec4Record {
    fn schema_name() -> &'static str {
        "Vec4"
    }
}

impl Vec4Record {
    pub fn w(&self) -> F32Field {
        F32Field::new(self.0.push("w"))
    }

    pub fn x(&self) -> F32Field {
        F32Field::new(self.0.push("x"))
    }

    pub fn y(&self) -> F32Field {
        F32Field::new(self.0.push("y"))
    }

    pub fn z(&self) -> F32Field {
        F32Field::new(self.0.push("z"))
    }
}
