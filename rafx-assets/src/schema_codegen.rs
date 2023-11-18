// This file generated automatically by hydrate-codegen. Do not make manual edits. Use include!() to place these types in the intended location.
#[derive(Default)]
pub struct ComputePipelineAssetAccessor(PropertyPath);

impl FieldAccessor for ComputePipelineAssetAccessor {
    fn new(property_path: PropertyPath) -> Self {
        ComputePipelineAssetAccessor(property_path)
    }
}

impl RecordAccessor for ComputePipelineAssetAccessor {
    fn schema_name() -> &'static str {
        "ComputePipelineAsset"
    }
}

impl ComputePipelineAssetAccessor {
    pub fn entry_name(&self) -> StringFieldAccessor {
        StringFieldAccessor::new(self.0.push("entry_name"))
    }

    pub fn shader_module(&self) -> AssetRefFieldAccessor {
        AssetRefFieldAccessor::new(self.0.push("shader_module"))
    }
}
pub struct ComputePipelineAssetReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for ComputePipelineAssetReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        ComputePipelineAssetReader(property_path, data_container)
    }
}

impl<'a> RecordReader for ComputePipelineAssetReader<'a> {
    fn schema_name() -> &'static str {
        "ComputePipelineAsset"
    }
}

impl<'a> ComputePipelineAssetReader<'a> {
    pub fn entry_name(&self) -> StringFieldReader {
        StringFieldReader::new(self.0.push("entry_name"), self.1)
    }

    pub fn shader_module(&self) -> AssetRefFieldReader {
        AssetRefFieldReader::new(self.0.push("shader_module"), self.1)
    }
}
pub struct ComputePipelineAssetWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for ComputePipelineAssetWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        ComputePipelineAssetWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for ComputePipelineAssetWriter<'a> {
    fn schema_name() -> &'static str {
        "ComputePipelineAsset"
    }
}

impl<'a> ComputePipelineAssetWriter<'a> {
    pub fn entry_name(self: &'a Self) -> StringFieldWriter {
        StringFieldWriter::new(self.0.push("entry_name"), &self.1)
    }

    pub fn shader_module(self: &'a Self) -> AssetRefFieldWriter {
        AssetRefFieldWriter::new(self.0.push("shader_module"), &self.1)
    }
}
pub struct ComputePipelineAssetOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for ComputePipelineAssetOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        ComputePipelineAssetOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for ComputePipelineAssetOwned {
    fn schema_name() -> &'static str {
        "ComputePipelineAsset"
    }
}

impl ComputePipelineAssetOwned {
    pub fn entry_name(self: &Self) -> StringFieldOwned {
        StringFieldOwned::new(self.0.push("entry_name"), &self.1)
    }

    pub fn shader_module(self: &Self) -> AssetRefFieldOwned {
        AssetRefFieldOwned::new(self.0.push("shader_module"), &self.1)
    }
}
#[derive(Default)]
pub struct GpuCompressedImageAssetAccessor(PropertyPath);

impl FieldAccessor for GpuCompressedImageAssetAccessor {
    fn new(property_path: PropertyPath) -> Self {
        GpuCompressedImageAssetAccessor(property_path)
    }
}

impl RecordAccessor for GpuCompressedImageAssetAccessor {
    fn schema_name() -> &'static str {
        "GpuCompressedImageAsset"
    }
}

impl GpuCompressedImageAssetAccessor {
}
pub struct GpuCompressedImageAssetReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for GpuCompressedImageAssetReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        GpuCompressedImageAssetReader(property_path, data_container)
    }
}

impl<'a> RecordReader for GpuCompressedImageAssetReader<'a> {
    fn schema_name() -> &'static str {
        "GpuCompressedImageAsset"
    }
}

impl<'a> GpuCompressedImageAssetReader<'a> {
}
pub struct GpuCompressedImageAssetWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for GpuCompressedImageAssetWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        GpuCompressedImageAssetWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for GpuCompressedImageAssetWriter<'a> {
    fn schema_name() -> &'static str {
        "GpuCompressedImageAsset"
    }
}

impl<'a> GpuCompressedImageAssetWriter<'a> {
}
pub struct GpuCompressedImageAssetOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for GpuCompressedImageAssetOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        GpuCompressedImageAssetOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for GpuCompressedImageAssetOwned {
    fn schema_name() -> &'static str {
        "GpuCompressedImageAsset"
    }
}

impl GpuCompressedImageAssetOwned {
}
#[derive(Default)]
pub struct GpuCompressedImageImportedDataAccessor(PropertyPath);

impl FieldAccessor for GpuCompressedImageImportedDataAccessor {
    fn new(property_path: PropertyPath) -> Self {
        GpuCompressedImageImportedDataAccessor(property_path)
    }
}

impl RecordAccessor for GpuCompressedImageImportedDataAccessor {
    fn schema_name() -> &'static str {
        "GpuCompressedImageImportedData"
    }
}

impl GpuCompressedImageImportedDataAccessor {
    pub fn data_layers(&self) -> DynamicArrayFieldAccessor::<GpuImageSubresourceLayerAccessor> {
        DynamicArrayFieldAccessor::<GpuImageSubresourceLayerAccessor>::new(self.0.push("data_layers"))
    }

    pub fn data_single_buffer(&self) -> BytesFieldAccessor {
        BytesFieldAccessor::new(self.0.push("data_single_buffer"))
    }

    pub fn format(&self) -> EnumFieldAccessor::<GpuImageAssetDataFormatEnum> {
        EnumFieldAccessor::<GpuImageAssetDataFormatEnum>::new(self.0.push("format"))
    }

    pub fn height(&self) -> U32FieldAccessor {
        U32FieldAccessor::new(self.0.push("height"))
    }

    pub fn is_cube_texture(&self) -> BooleanFieldAccessor {
        BooleanFieldAccessor::new(self.0.push("is_cube_texture"))
    }

    pub fn width(&self) -> U32FieldAccessor {
        U32FieldAccessor::new(self.0.push("width"))
    }
}
pub struct GpuCompressedImageImportedDataReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for GpuCompressedImageImportedDataReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        GpuCompressedImageImportedDataReader(property_path, data_container)
    }
}

impl<'a> RecordReader for GpuCompressedImageImportedDataReader<'a> {
    fn schema_name() -> &'static str {
        "GpuCompressedImageImportedData"
    }
}

impl<'a> GpuCompressedImageImportedDataReader<'a> {
    pub fn data_layers(&self) -> DynamicArrayFieldReader::<GpuImageSubresourceLayerReader> {
        DynamicArrayFieldReader::<GpuImageSubresourceLayerReader>::new(self.0.push("data_layers"), self.1)
    }

    pub fn data_single_buffer(&self) -> BytesFieldReader {
        BytesFieldReader::new(self.0.push("data_single_buffer"), self.1)
    }

    pub fn format(&self) -> EnumFieldReader::<GpuImageAssetDataFormatEnum> {
        EnumFieldReader::<GpuImageAssetDataFormatEnum>::new(self.0.push("format"), self.1)
    }

    pub fn height(&self) -> U32FieldReader {
        U32FieldReader::new(self.0.push("height"), self.1)
    }

    pub fn is_cube_texture(&self) -> BooleanFieldReader {
        BooleanFieldReader::new(self.0.push("is_cube_texture"), self.1)
    }

    pub fn width(&self) -> U32FieldReader {
        U32FieldReader::new(self.0.push("width"), self.1)
    }
}
pub struct GpuCompressedImageImportedDataWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for GpuCompressedImageImportedDataWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        GpuCompressedImageImportedDataWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for GpuCompressedImageImportedDataWriter<'a> {
    fn schema_name() -> &'static str {
        "GpuCompressedImageImportedData"
    }
}

impl<'a> GpuCompressedImageImportedDataWriter<'a> {
    pub fn data_layers(self: &'a Self) -> DynamicArrayFieldWriter::<GpuImageSubresourceLayerWriter> {
        DynamicArrayFieldWriter::<GpuImageSubresourceLayerWriter>::new(self.0.push("data_layers"), &self.1)
    }

    pub fn data_single_buffer(self: &'a Self) -> BytesFieldWriter {
        BytesFieldWriter::new(self.0.push("data_single_buffer"), &self.1)
    }

    pub fn format(self: &'a Self) -> EnumFieldWriter::<GpuImageAssetDataFormatEnum> {
        EnumFieldWriter::<GpuImageAssetDataFormatEnum>::new(self.0.push("format"), &self.1)
    }

    pub fn height(self: &'a Self) -> U32FieldWriter {
        U32FieldWriter::new(self.0.push("height"), &self.1)
    }

    pub fn is_cube_texture(self: &'a Self) -> BooleanFieldWriter {
        BooleanFieldWriter::new(self.0.push("is_cube_texture"), &self.1)
    }

    pub fn width(self: &'a Self) -> U32FieldWriter {
        U32FieldWriter::new(self.0.push("width"), &self.1)
    }
}
pub struct GpuCompressedImageImportedDataOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for GpuCompressedImageImportedDataOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        GpuCompressedImageImportedDataOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for GpuCompressedImageImportedDataOwned {
    fn schema_name() -> &'static str {
        "GpuCompressedImageImportedData"
    }
}

impl GpuCompressedImageImportedDataOwned {
    pub fn data_layers(self: &Self) -> DynamicArrayFieldOwned::<GpuImageSubresourceLayerOwned> {
        DynamicArrayFieldOwned::<GpuImageSubresourceLayerOwned>::new(self.0.push("data_layers"), &self.1)
    }

    pub fn data_single_buffer(self: &Self) -> BytesFieldOwned {
        BytesFieldOwned::new(self.0.push("data_single_buffer"), &self.1)
    }

    pub fn format(self: &Self) -> EnumFieldOwned::<GpuImageAssetDataFormatEnum> {
        EnumFieldOwned::<GpuImageAssetDataFormatEnum>::new(self.0.push("format"), &self.1)
    }

    pub fn height(self: &Self) -> U32FieldOwned {
        U32FieldOwned::new(self.0.push("height"), &self.1)
    }

    pub fn is_cube_texture(self: &Self) -> BooleanFieldOwned {
        BooleanFieldOwned::new(self.0.push("is_cube_texture"), &self.1)
    }

    pub fn width(self: &Self) -> U32FieldOwned {
        U32FieldOwned::new(self.0.push("width"), &self.1)
    }
}
#[derive(Default)]
pub struct GpuImageAssetAccessor(PropertyPath);

impl FieldAccessor for GpuImageAssetAccessor {
    fn new(property_path: PropertyPath) -> Self {
        GpuImageAssetAccessor(property_path)
    }
}

impl RecordAccessor for GpuImageAssetAccessor {
    fn schema_name() -> &'static str {
        "GpuImageAsset"
    }
}

impl GpuImageAssetAccessor {
    pub fn basis_compression(&self) -> BooleanFieldAccessor {
        BooleanFieldAccessor::new(self.0.push("basis_compression"))
    }

    pub fn basis_compression_settings(&self) -> GpuImageBasisCompressionSettingsAccessor {
        GpuImageBasisCompressionSettingsAccessor::new(self.0.push("basis_compression_settings"))
    }

    pub fn color_space(&self) -> EnumFieldAccessor::<GpuImageColorSpaceEnum> {
        EnumFieldAccessor::<GpuImageColorSpaceEnum>::new(self.0.push("color_space"))
    }

    pub fn mip_generation(&self) -> EnumFieldAccessor::<GpuImageMipGenerationEnum> {
        EnumFieldAccessor::<GpuImageMipGenerationEnum>::new(self.0.push("mip_generation"))
    }
}
pub struct GpuImageAssetReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for GpuImageAssetReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        GpuImageAssetReader(property_path, data_container)
    }
}

impl<'a> RecordReader for GpuImageAssetReader<'a> {
    fn schema_name() -> &'static str {
        "GpuImageAsset"
    }
}

impl<'a> GpuImageAssetReader<'a> {
    pub fn basis_compression(&self) -> BooleanFieldReader {
        BooleanFieldReader::new(self.0.push("basis_compression"), self.1)
    }

    pub fn basis_compression_settings(&self) -> GpuImageBasisCompressionSettingsReader {
        GpuImageBasisCompressionSettingsReader::new(self.0.push("basis_compression_settings"), self.1)
    }

    pub fn color_space(&self) -> EnumFieldReader::<GpuImageColorSpaceEnum> {
        EnumFieldReader::<GpuImageColorSpaceEnum>::new(self.0.push("color_space"), self.1)
    }

    pub fn mip_generation(&self) -> EnumFieldReader::<GpuImageMipGenerationEnum> {
        EnumFieldReader::<GpuImageMipGenerationEnum>::new(self.0.push("mip_generation"), self.1)
    }
}
pub struct GpuImageAssetWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for GpuImageAssetWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        GpuImageAssetWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for GpuImageAssetWriter<'a> {
    fn schema_name() -> &'static str {
        "GpuImageAsset"
    }
}

impl<'a> GpuImageAssetWriter<'a> {
    pub fn basis_compression(self: &'a Self) -> BooleanFieldWriter {
        BooleanFieldWriter::new(self.0.push("basis_compression"), &self.1)
    }

    pub fn basis_compression_settings(self: &'a Self) -> GpuImageBasisCompressionSettingsWriter {
        GpuImageBasisCompressionSettingsWriter::new(self.0.push("basis_compression_settings"), &self.1)
    }

    pub fn color_space(self: &'a Self) -> EnumFieldWriter::<GpuImageColorSpaceEnum> {
        EnumFieldWriter::<GpuImageColorSpaceEnum>::new(self.0.push("color_space"), &self.1)
    }

    pub fn mip_generation(self: &'a Self) -> EnumFieldWriter::<GpuImageMipGenerationEnum> {
        EnumFieldWriter::<GpuImageMipGenerationEnum>::new(self.0.push("mip_generation"), &self.1)
    }
}
pub struct GpuImageAssetOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for GpuImageAssetOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        GpuImageAssetOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for GpuImageAssetOwned {
    fn schema_name() -> &'static str {
        "GpuImageAsset"
    }
}

impl GpuImageAssetOwned {
    pub fn basis_compression(self: &Self) -> BooleanFieldOwned {
        BooleanFieldOwned::new(self.0.push("basis_compression"), &self.1)
    }

    pub fn basis_compression_settings(self: &Self) -> GpuImageBasisCompressionSettingsOwned {
        GpuImageBasisCompressionSettingsOwned::new(self.0.push("basis_compression_settings"), &self.1)
    }

    pub fn color_space(self: &Self) -> EnumFieldOwned::<GpuImageColorSpaceEnum> {
        EnumFieldOwned::<GpuImageColorSpaceEnum>::new(self.0.push("color_space"), &self.1)
    }

    pub fn mip_generation(self: &Self) -> EnumFieldOwned::<GpuImageMipGenerationEnum> {
        EnumFieldOwned::<GpuImageMipGenerationEnum>::new(self.0.push("mip_generation"), &self.1)
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
pub struct GpuImageBasisCompressionSettingsAccessor(PropertyPath);

impl FieldAccessor for GpuImageBasisCompressionSettingsAccessor {
    fn new(property_path: PropertyPath) -> Self {
        GpuImageBasisCompressionSettingsAccessor(property_path)
    }
}

impl RecordAccessor for GpuImageBasisCompressionSettingsAccessor {
    fn schema_name() -> &'static str {
        "GpuImageBasisCompressionSettings"
    }
}

impl GpuImageBasisCompressionSettingsAccessor {
    pub fn compression_type(&self) -> EnumFieldAccessor::<GpuImageBasisCompressionTypeEnum> {
        EnumFieldAccessor::<GpuImageBasisCompressionTypeEnum>::new(self.0.push("compression_type"))
    }

    pub fn quality(&self) -> U32FieldAccessor {
        U32FieldAccessor::new(self.0.push("quality"))
    }
}
pub struct GpuImageBasisCompressionSettingsReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for GpuImageBasisCompressionSettingsReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        GpuImageBasisCompressionSettingsReader(property_path, data_container)
    }
}

impl<'a> RecordReader for GpuImageBasisCompressionSettingsReader<'a> {
    fn schema_name() -> &'static str {
        "GpuImageBasisCompressionSettings"
    }
}

impl<'a> GpuImageBasisCompressionSettingsReader<'a> {
    pub fn compression_type(&self) -> EnumFieldReader::<GpuImageBasisCompressionTypeEnum> {
        EnumFieldReader::<GpuImageBasisCompressionTypeEnum>::new(self.0.push("compression_type"), self.1)
    }

    pub fn quality(&self) -> U32FieldReader {
        U32FieldReader::new(self.0.push("quality"), self.1)
    }
}
pub struct GpuImageBasisCompressionSettingsWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for GpuImageBasisCompressionSettingsWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        GpuImageBasisCompressionSettingsWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for GpuImageBasisCompressionSettingsWriter<'a> {
    fn schema_name() -> &'static str {
        "GpuImageBasisCompressionSettings"
    }
}

impl<'a> GpuImageBasisCompressionSettingsWriter<'a> {
    pub fn compression_type(self: &'a Self) -> EnumFieldWriter::<GpuImageBasisCompressionTypeEnum> {
        EnumFieldWriter::<GpuImageBasisCompressionTypeEnum>::new(self.0.push("compression_type"), &self.1)
    }

    pub fn quality(self: &'a Self) -> U32FieldWriter {
        U32FieldWriter::new(self.0.push("quality"), &self.1)
    }
}
pub struct GpuImageBasisCompressionSettingsOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for GpuImageBasisCompressionSettingsOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        GpuImageBasisCompressionSettingsOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for GpuImageBasisCompressionSettingsOwned {
    fn schema_name() -> &'static str {
        "GpuImageBasisCompressionSettings"
    }
}

impl GpuImageBasisCompressionSettingsOwned {
    pub fn compression_type(self: &Self) -> EnumFieldOwned::<GpuImageBasisCompressionTypeEnum> {
        EnumFieldOwned::<GpuImageBasisCompressionTypeEnum>::new(self.0.push("compression_type"), &self.1)
    }

    pub fn quality(self: &Self) -> U32FieldOwned {
        U32FieldOwned::new(self.0.push("quality"), &self.1)
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
pub struct GpuImageImportedDataAccessor(PropertyPath);

impl FieldAccessor for GpuImageImportedDataAccessor {
    fn new(property_path: PropertyPath) -> Self {
        GpuImageImportedDataAccessor(property_path)
    }
}

impl RecordAccessor for GpuImageImportedDataAccessor {
    fn schema_name() -> &'static str {
        "GpuImageImportedData"
    }
}

impl GpuImageImportedDataAccessor {
    pub fn height(&self) -> U32FieldAccessor {
        U32FieldAccessor::new(self.0.push("height"))
    }

    pub fn image_bytes(&self) -> BytesFieldAccessor {
        BytesFieldAccessor::new(self.0.push("image_bytes"))
    }

    pub fn width(&self) -> U32FieldAccessor {
        U32FieldAccessor::new(self.0.push("width"))
    }
}
pub struct GpuImageImportedDataReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for GpuImageImportedDataReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        GpuImageImportedDataReader(property_path, data_container)
    }
}

impl<'a> RecordReader for GpuImageImportedDataReader<'a> {
    fn schema_name() -> &'static str {
        "GpuImageImportedData"
    }
}

impl<'a> GpuImageImportedDataReader<'a> {
    pub fn height(&self) -> U32FieldReader {
        U32FieldReader::new(self.0.push("height"), self.1)
    }

    pub fn image_bytes(&self) -> BytesFieldReader {
        BytesFieldReader::new(self.0.push("image_bytes"), self.1)
    }

    pub fn width(&self) -> U32FieldReader {
        U32FieldReader::new(self.0.push("width"), self.1)
    }
}
pub struct GpuImageImportedDataWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for GpuImageImportedDataWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        GpuImageImportedDataWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for GpuImageImportedDataWriter<'a> {
    fn schema_name() -> &'static str {
        "GpuImageImportedData"
    }
}

impl<'a> GpuImageImportedDataWriter<'a> {
    pub fn height(self: &'a Self) -> U32FieldWriter {
        U32FieldWriter::new(self.0.push("height"), &self.1)
    }

    pub fn image_bytes(self: &'a Self) -> BytesFieldWriter {
        BytesFieldWriter::new(self.0.push("image_bytes"), &self.1)
    }

    pub fn width(self: &'a Self) -> U32FieldWriter {
        U32FieldWriter::new(self.0.push("width"), &self.1)
    }
}
pub struct GpuImageImportedDataOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for GpuImageImportedDataOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        GpuImageImportedDataOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for GpuImageImportedDataOwned {
    fn schema_name() -> &'static str {
        "GpuImageImportedData"
    }
}

impl GpuImageImportedDataOwned {
    pub fn height(self: &Self) -> U32FieldOwned {
        U32FieldOwned::new(self.0.push("height"), &self.1)
    }

    pub fn image_bytes(self: &Self) -> BytesFieldOwned {
        BytesFieldOwned::new(self.0.push("image_bytes"), &self.1)
    }

    pub fn width(self: &Self) -> U32FieldOwned {
        U32FieldOwned::new(self.0.push("width"), &self.1)
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
pub struct GpuImageSubresourceLayerAccessor(PropertyPath);

impl FieldAccessor for GpuImageSubresourceLayerAccessor {
    fn new(property_path: PropertyPath) -> Self {
        GpuImageSubresourceLayerAccessor(property_path)
    }
}

impl RecordAccessor for GpuImageSubresourceLayerAccessor {
    fn schema_name() -> &'static str {
        "GpuImageSubresourceLayer"
    }
}

impl GpuImageSubresourceLayerAccessor {
    pub fn mip_levels(&self) -> DynamicArrayFieldAccessor::<GpuImageSubresourceMipLevelAccessor> {
        DynamicArrayFieldAccessor::<GpuImageSubresourceMipLevelAccessor>::new(self.0.push("mip_levels"))
    }
}
pub struct GpuImageSubresourceLayerReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for GpuImageSubresourceLayerReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        GpuImageSubresourceLayerReader(property_path, data_container)
    }
}

impl<'a> RecordReader for GpuImageSubresourceLayerReader<'a> {
    fn schema_name() -> &'static str {
        "GpuImageSubresourceLayer"
    }
}

impl<'a> GpuImageSubresourceLayerReader<'a> {
    pub fn mip_levels(&self) -> DynamicArrayFieldReader::<GpuImageSubresourceMipLevelReader> {
        DynamicArrayFieldReader::<GpuImageSubresourceMipLevelReader>::new(self.0.push("mip_levels"), self.1)
    }
}
pub struct GpuImageSubresourceLayerWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for GpuImageSubresourceLayerWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        GpuImageSubresourceLayerWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for GpuImageSubresourceLayerWriter<'a> {
    fn schema_name() -> &'static str {
        "GpuImageSubresourceLayer"
    }
}

impl<'a> GpuImageSubresourceLayerWriter<'a> {
    pub fn mip_levels(self: &'a Self) -> DynamicArrayFieldWriter::<GpuImageSubresourceMipLevelWriter> {
        DynamicArrayFieldWriter::<GpuImageSubresourceMipLevelWriter>::new(self.0.push("mip_levels"), &self.1)
    }
}
pub struct GpuImageSubresourceLayerOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for GpuImageSubresourceLayerOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        GpuImageSubresourceLayerOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for GpuImageSubresourceLayerOwned {
    fn schema_name() -> &'static str {
        "GpuImageSubresourceLayer"
    }
}

impl GpuImageSubresourceLayerOwned {
    pub fn mip_levels(self: &Self) -> DynamicArrayFieldOwned::<GpuImageSubresourceMipLevelOwned> {
        DynamicArrayFieldOwned::<GpuImageSubresourceMipLevelOwned>::new(self.0.push("mip_levels"), &self.1)
    }
}
#[derive(Default)]
pub struct GpuImageSubresourceMipLevelAccessor(PropertyPath);

impl FieldAccessor for GpuImageSubresourceMipLevelAccessor {
    fn new(property_path: PropertyPath) -> Self {
        GpuImageSubresourceMipLevelAccessor(property_path)
    }
}

impl RecordAccessor for GpuImageSubresourceMipLevelAccessor {
    fn schema_name() -> &'static str {
        "GpuImageSubresourceMipLevel"
    }
}

impl GpuImageSubresourceMipLevelAccessor {
    pub fn bytes(&self) -> BytesFieldAccessor {
        BytesFieldAccessor::new(self.0.push("bytes"))
    }

    pub fn height(&self) -> U32FieldAccessor {
        U32FieldAccessor::new(self.0.push("height"))
    }

    pub fn width(&self) -> U32FieldAccessor {
        U32FieldAccessor::new(self.0.push("width"))
    }
}
pub struct GpuImageSubresourceMipLevelReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for GpuImageSubresourceMipLevelReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        GpuImageSubresourceMipLevelReader(property_path, data_container)
    }
}

impl<'a> RecordReader for GpuImageSubresourceMipLevelReader<'a> {
    fn schema_name() -> &'static str {
        "GpuImageSubresourceMipLevel"
    }
}

impl<'a> GpuImageSubresourceMipLevelReader<'a> {
    pub fn bytes(&self) -> BytesFieldReader {
        BytesFieldReader::new(self.0.push("bytes"), self.1)
    }

    pub fn height(&self) -> U32FieldReader {
        U32FieldReader::new(self.0.push("height"), self.1)
    }

    pub fn width(&self) -> U32FieldReader {
        U32FieldReader::new(self.0.push("width"), self.1)
    }
}
pub struct GpuImageSubresourceMipLevelWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for GpuImageSubresourceMipLevelWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        GpuImageSubresourceMipLevelWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for GpuImageSubresourceMipLevelWriter<'a> {
    fn schema_name() -> &'static str {
        "GpuImageSubresourceMipLevel"
    }
}

impl<'a> GpuImageSubresourceMipLevelWriter<'a> {
    pub fn bytes(self: &'a Self) -> BytesFieldWriter {
        BytesFieldWriter::new(self.0.push("bytes"), &self.1)
    }

    pub fn height(self: &'a Self) -> U32FieldWriter {
        U32FieldWriter::new(self.0.push("height"), &self.1)
    }

    pub fn width(self: &'a Self) -> U32FieldWriter {
        U32FieldWriter::new(self.0.push("width"), &self.1)
    }
}
pub struct GpuImageSubresourceMipLevelOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for GpuImageSubresourceMipLevelOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        GpuImageSubresourceMipLevelOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for GpuImageSubresourceMipLevelOwned {
    fn schema_name() -> &'static str {
        "GpuImageSubresourceMipLevel"
    }
}

impl GpuImageSubresourceMipLevelOwned {
    pub fn bytes(self: &Self) -> BytesFieldOwned {
        BytesFieldOwned::new(self.0.push("bytes"), &self.1)
    }

    pub fn height(self: &Self) -> U32FieldOwned {
        U32FieldOwned::new(self.0.push("height"), &self.1)
    }

    pub fn width(self: &Self) -> U32FieldOwned {
        U32FieldOwned::new(self.0.push("width"), &self.1)
    }
}
#[derive(Default)]
pub struct GraphicsPipelineShaderStageAccessor(PropertyPath);

impl FieldAccessor for GraphicsPipelineShaderStageAccessor {
    fn new(property_path: PropertyPath) -> Self {
        GraphicsPipelineShaderStageAccessor(property_path)
    }
}

impl RecordAccessor for GraphicsPipelineShaderStageAccessor {
    fn schema_name() -> &'static str {
        "GraphicsPipelineShaderStage"
    }
}

impl GraphicsPipelineShaderStageAccessor {
    pub fn entry_name(&self) -> StringFieldAccessor {
        StringFieldAccessor::new(self.0.push("entry_name"))
    }

    pub fn shader_module(&self) -> AssetRefFieldAccessor {
        AssetRefFieldAccessor::new(self.0.push("shader_module"))
    }
}
pub struct GraphicsPipelineShaderStageReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for GraphicsPipelineShaderStageReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        GraphicsPipelineShaderStageReader(property_path, data_container)
    }
}

impl<'a> RecordReader for GraphicsPipelineShaderStageReader<'a> {
    fn schema_name() -> &'static str {
        "GraphicsPipelineShaderStage"
    }
}

impl<'a> GraphicsPipelineShaderStageReader<'a> {
    pub fn entry_name(&self) -> StringFieldReader {
        StringFieldReader::new(self.0.push("entry_name"), self.1)
    }

    pub fn shader_module(&self) -> AssetRefFieldReader {
        AssetRefFieldReader::new(self.0.push("shader_module"), self.1)
    }
}
pub struct GraphicsPipelineShaderStageWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for GraphicsPipelineShaderStageWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        GraphicsPipelineShaderStageWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for GraphicsPipelineShaderStageWriter<'a> {
    fn schema_name() -> &'static str {
        "GraphicsPipelineShaderStage"
    }
}

impl<'a> GraphicsPipelineShaderStageWriter<'a> {
    pub fn entry_name(self: &'a Self) -> StringFieldWriter {
        StringFieldWriter::new(self.0.push("entry_name"), &self.1)
    }

    pub fn shader_module(self: &'a Self) -> AssetRefFieldWriter {
        AssetRefFieldWriter::new(self.0.push("shader_module"), &self.1)
    }
}
pub struct GraphicsPipelineShaderStageOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for GraphicsPipelineShaderStageOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        GraphicsPipelineShaderStageOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for GraphicsPipelineShaderStageOwned {
    fn schema_name() -> &'static str {
        "GraphicsPipelineShaderStage"
    }
}

impl GraphicsPipelineShaderStageOwned {
    pub fn entry_name(self: &Self) -> StringFieldOwned {
        StringFieldOwned::new(self.0.push("entry_name"), &self.1)
    }

    pub fn shader_module(self: &Self) -> AssetRefFieldOwned {
        AssetRefFieldOwned::new(self.0.push("shader_module"), &self.1)
    }
}
#[derive(Default)]
pub struct MaterialAssetAccessor(PropertyPath);

impl FieldAccessor for MaterialAssetAccessor {
    fn new(property_path: PropertyPath) -> Self {
        MaterialAssetAccessor(property_path)
    }
}

impl RecordAccessor for MaterialAssetAccessor {
    fn schema_name() -> &'static str {
        "MaterialAsset"
    }
}

impl MaterialAssetAccessor {
    pub fn passes(&self) -> DynamicArrayFieldAccessor::<MaterialPassAccessor> {
        DynamicArrayFieldAccessor::<MaterialPassAccessor>::new(self.0.push("passes"))
    }
}
pub struct MaterialAssetReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for MaterialAssetReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MaterialAssetReader(property_path, data_container)
    }
}

impl<'a> RecordReader for MaterialAssetReader<'a> {
    fn schema_name() -> &'static str {
        "MaterialAsset"
    }
}

impl<'a> MaterialAssetReader<'a> {
    pub fn passes(&self) -> DynamicArrayFieldReader::<MaterialPassReader> {
        DynamicArrayFieldReader::<MaterialPassReader>::new(self.0.push("passes"), self.1)
    }
}
pub struct MaterialAssetWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for MaterialAssetWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MaterialAssetWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for MaterialAssetWriter<'a> {
    fn schema_name() -> &'static str {
        "MaterialAsset"
    }
}

impl<'a> MaterialAssetWriter<'a> {
    pub fn passes(self: &'a Self) -> DynamicArrayFieldWriter::<MaterialPassWriter> {
        DynamicArrayFieldWriter::<MaterialPassWriter>::new(self.0.push("passes"), &self.1)
    }
}
pub struct MaterialAssetOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for MaterialAssetOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        MaterialAssetOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for MaterialAssetOwned {
    fn schema_name() -> &'static str {
        "MaterialAsset"
    }
}

impl MaterialAssetOwned {
    pub fn passes(self: &Self) -> DynamicArrayFieldOwned::<MaterialPassOwned> {
        DynamicArrayFieldOwned::<MaterialPassOwned>::new(self.0.push("passes"), &self.1)
    }
}
#[derive(Default)]
pub struct MaterialInstanceAssetAccessor(PropertyPath);

impl FieldAccessor for MaterialInstanceAssetAccessor {
    fn new(property_path: PropertyPath) -> Self {
        MaterialInstanceAssetAccessor(property_path)
    }
}

impl RecordAccessor for MaterialInstanceAssetAccessor {
    fn schema_name() -> &'static str {
        "MaterialInstanceAsset"
    }
}

impl MaterialInstanceAssetAccessor {
    pub fn material(&self) -> AssetRefFieldAccessor {
        AssetRefFieldAccessor::new(self.0.push("material"))
    }

    pub fn slot_assignments(&self) -> DynamicArrayFieldAccessor::<MaterialInstanceSlotAssignmentAccessor> {
        DynamicArrayFieldAccessor::<MaterialInstanceSlotAssignmentAccessor>::new(self.0.push("slot_assignments"))
    }
}
pub struct MaterialInstanceAssetReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for MaterialInstanceAssetReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MaterialInstanceAssetReader(property_path, data_container)
    }
}

impl<'a> RecordReader for MaterialInstanceAssetReader<'a> {
    fn schema_name() -> &'static str {
        "MaterialInstanceAsset"
    }
}

impl<'a> MaterialInstanceAssetReader<'a> {
    pub fn material(&self) -> AssetRefFieldReader {
        AssetRefFieldReader::new(self.0.push("material"), self.1)
    }

    pub fn slot_assignments(&self) -> DynamicArrayFieldReader::<MaterialInstanceSlotAssignmentReader> {
        DynamicArrayFieldReader::<MaterialInstanceSlotAssignmentReader>::new(self.0.push("slot_assignments"), self.1)
    }
}
pub struct MaterialInstanceAssetWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for MaterialInstanceAssetWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MaterialInstanceAssetWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for MaterialInstanceAssetWriter<'a> {
    fn schema_name() -> &'static str {
        "MaterialInstanceAsset"
    }
}

impl<'a> MaterialInstanceAssetWriter<'a> {
    pub fn material(self: &'a Self) -> AssetRefFieldWriter {
        AssetRefFieldWriter::new(self.0.push("material"), &self.1)
    }

    pub fn slot_assignments(self: &'a Self) -> DynamicArrayFieldWriter::<MaterialInstanceSlotAssignmentWriter> {
        DynamicArrayFieldWriter::<MaterialInstanceSlotAssignmentWriter>::new(self.0.push("slot_assignments"), &self.1)
    }
}
pub struct MaterialInstanceAssetOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for MaterialInstanceAssetOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        MaterialInstanceAssetOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for MaterialInstanceAssetOwned {
    fn schema_name() -> &'static str {
        "MaterialInstanceAsset"
    }
}

impl MaterialInstanceAssetOwned {
    pub fn material(self: &Self) -> AssetRefFieldOwned {
        AssetRefFieldOwned::new(self.0.push("material"), &self.1)
    }

    pub fn slot_assignments(self: &Self) -> DynamicArrayFieldOwned::<MaterialInstanceSlotAssignmentOwned> {
        DynamicArrayFieldOwned::<MaterialInstanceSlotAssignmentOwned>::new(self.0.push("slot_assignments"), &self.1)
    }
}
#[derive(Default)]
pub struct MaterialInstanceSlotAssignmentAccessor(PropertyPath);

impl FieldAccessor for MaterialInstanceSlotAssignmentAccessor {
    fn new(property_path: PropertyPath) -> Self {
        MaterialInstanceSlotAssignmentAccessor(property_path)
    }
}

impl RecordAccessor for MaterialInstanceSlotAssignmentAccessor {
    fn schema_name() -> &'static str {
        "MaterialInstanceSlotAssignment"
    }
}

impl MaterialInstanceSlotAssignmentAccessor {
    pub fn array_index(&self) -> U32FieldAccessor {
        U32FieldAccessor::new(self.0.push("array_index"))
    }

    pub fn buffer_data(&self) -> NullableFieldAccessor::<BytesFieldAccessor> {
        NullableFieldAccessor::<BytesFieldAccessor>::new(self.0.push("buffer_data"))
    }

    pub fn image(&self) -> AssetRefFieldAccessor {
        AssetRefFieldAccessor::new(self.0.push("image"))
    }

    pub fn sampler(&self) -> StringFieldAccessor {
        StringFieldAccessor::new(self.0.push("sampler"))
    }

    pub fn slot_name(&self) -> StringFieldAccessor {
        StringFieldAccessor::new(self.0.push("slot_name"))
    }
}
pub struct MaterialInstanceSlotAssignmentReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for MaterialInstanceSlotAssignmentReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MaterialInstanceSlotAssignmentReader(property_path, data_container)
    }
}

impl<'a> RecordReader for MaterialInstanceSlotAssignmentReader<'a> {
    fn schema_name() -> &'static str {
        "MaterialInstanceSlotAssignment"
    }
}

impl<'a> MaterialInstanceSlotAssignmentReader<'a> {
    pub fn array_index(&self) -> U32FieldReader {
        U32FieldReader::new(self.0.push("array_index"), self.1)
    }

    pub fn buffer_data(&self) -> NullableFieldReader::<BytesFieldReader> {
        NullableFieldReader::<BytesFieldReader>::new(self.0.push("buffer_data"), self.1)
    }

    pub fn image(&self) -> AssetRefFieldReader {
        AssetRefFieldReader::new(self.0.push("image"), self.1)
    }

    pub fn sampler(&self) -> StringFieldReader {
        StringFieldReader::new(self.0.push("sampler"), self.1)
    }

    pub fn slot_name(&self) -> StringFieldReader {
        StringFieldReader::new(self.0.push("slot_name"), self.1)
    }
}
pub struct MaterialInstanceSlotAssignmentWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for MaterialInstanceSlotAssignmentWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MaterialInstanceSlotAssignmentWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for MaterialInstanceSlotAssignmentWriter<'a> {
    fn schema_name() -> &'static str {
        "MaterialInstanceSlotAssignment"
    }
}

impl<'a> MaterialInstanceSlotAssignmentWriter<'a> {
    pub fn array_index(self: &'a Self) -> U32FieldWriter {
        U32FieldWriter::new(self.0.push("array_index"), &self.1)
    }

    pub fn buffer_data(self: &'a Self) -> NullableFieldWriter::<BytesFieldWriter> {
        NullableFieldWriter::<BytesFieldWriter>::new(self.0.push("buffer_data"), &self.1)
    }

    pub fn image(self: &'a Self) -> AssetRefFieldWriter {
        AssetRefFieldWriter::new(self.0.push("image"), &self.1)
    }

    pub fn sampler(self: &'a Self) -> StringFieldWriter {
        StringFieldWriter::new(self.0.push("sampler"), &self.1)
    }

    pub fn slot_name(self: &'a Self) -> StringFieldWriter {
        StringFieldWriter::new(self.0.push("slot_name"), &self.1)
    }
}
pub struct MaterialInstanceSlotAssignmentOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for MaterialInstanceSlotAssignmentOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        MaterialInstanceSlotAssignmentOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for MaterialInstanceSlotAssignmentOwned {
    fn schema_name() -> &'static str {
        "MaterialInstanceSlotAssignment"
    }
}

impl MaterialInstanceSlotAssignmentOwned {
    pub fn array_index(self: &Self) -> U32FieldOwned {
        U32FieldOwned::new(self.0.push("array_index"), &self.1)
    }

    pub fn buffer_data(self: &Self) -> NullableFieldOwned::<BytesFieldOwned> {
        NullableFieldOwned::<BytesFieldOwned>::new(self.0.push("buffer_data"), &self.1)
    }

    pub fn image(self: &Self) -> AssetRefFieldOwned {
        AssetRefFieldOwned::new(self.0.push("image"), &self.1)
    }

    pub fn sampler(self: &Self) -> StringFieldOwned {
        StringFieldOwned::new(self.0.push("sampler"), &self.1)
    }

    pub fn slot_name(self: &Self) -> StringFieldOwned {
        StringFieldOwned::new(self.0.push("slot_name"), &self.1)
    }
}
#[derive(Default)]
pub struct MaterialPassAccessor(PropertyPath);

impl FieldAccessor for MaterialPassAccessor {
    fn new(property_path: PropertyPath) -> Self {
        MaterialPassAccessor(property_path)
    }
}

impl RecordAccessor for MaterialPassAccessor {
    fn schema_name() -> &'static str {
        "MaterialPass"
    }
}

impl MaterialPassAccessor {
    pub fn fixed_function_state(&self) -> StringFieldAccessor {
        StringFieldAccessor::new(self.0.push("fixed_function_state"))
    }

    pub fn fragment_stage(&self) -> GraphicsPipelineShaderStageAccessor {
        GraphicsPipelineShaderStageAccessor::new(self.0.push("fragment_stage"))
    }

    pub fn name(&self) -> StringFieldAccessor {
        StringFieldAccessor::new(self.0.push("name"))
    }

    pub fn phase(&self) -> StringFieldAccessor {
        StringFieldAccessor::new(self.0.push("phase"))
    }

    pub fn vertex_stage(&self) -> GraphicsPipelineShaderStageAccessor {
        GraphicsPipelineShaderStageAccessor::new(self.0.push("vertex_stage"))
    }
}
pub struct MaterialPassReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for MaterialPassReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MaterialPassReader(property_path, data_container)
    }
}

impl<'a> RecordReader for MaterialPassReader<'a> {
    fn schema_name() -> &'static str {
        "MaterialPass"
    }
}

impl<'a> MaterialPassReader<'a> {
    pub fn fixed_function_state(&self) -> StringFieldReader {
        StringFieldReader::new(self.0.push("fixed_function_state"), self.1)
    }

    pub fn fragment_stage(&self) -> GraphicsPipelineShaderStageReader {
        GraphicsPipelineShaderStageReader::new(self.0.push("fragment_stage"), self.1)
    }

    pub fn name(&self) -> StringFieldReader {
        StringFieldReader::new(self.0.push("name"), self.1)
    }

    pub fn phase(&self) -> StringFieldReader {
        StringFieldReader::new(self.0.push("phase"), self.1)
    }

    pub fn vertex_stage(&self) -> GraphicsPipelineShaderStageReader {
        GraphicsPipelineShaderStageReader::new(self.0.push("vertex_stage"), self.1)
    }
}
pub struct MaterialPassWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for MaterialPassWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MaterialPassWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for MaterialPassWriter<'a> {
    fn schema_name() -> &'static str {
        "MaterialPass"
    }
}

impl<'a> MaterialPassWriter<'a> {
    pub fn fixed_function_state(self: &'a Self) -> StringFieldWriter {
        StringFieldWriter::new(self.0.push("fixed_function_state"), &self.1)
    }

    pub fn fragment_stage(self: &'a Self) -> GraphicsPipelineShaderStageWriter {
        GraphicsPipelineShaderStageWriter::new(self.0.push("fragment_stage"), &self.1)
    }

    pub fn name(self: &'a Self) -> StringFieldWriter {
        StringFieldWriter::new(self.0.push("name"), &self.1)
    }

    pub fn phase(self: &'a Self) -> StringFieldWriter {
        StringFieldWriter::new(self.0.push("phase"), &self.1)
    }

    pub fn vertex_stage(self: &'a Self) -> GraphicsPipelineShaderStageWriter {
        GraphicsPipelineShaderStageWriter::new(self.0.push("vertex_stage"), &self.1)
    }
}
pub struct MaterialPassOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for MaterialPassOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        MaterialPassOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for MaterialPassOwned {
    fn schema_name() -> &'static str {
        "MaterialPass"
    }
}

impl MaterialPassOwned {
    pub fn fixed_function_state(self: &Self) -> StringFieldOwned {
        StringFieldOwned::new(self.0.push("fixed_function_state"), &self.1)
    }

    pub fn fragment_stage(self: &Self) -> GraphicsPipelineShaderStageOwned {
        GraphicsPipelineShaderStageOwned::new(self.0.push("fragment_stage"), &self.1)
    }

    pub fn name(self: &Self) -> StringFieldOwned {
        StringFieldOwned::new(self.0.push("name"), &self.1)
    }

    pub fn phase(self: &Self) -> StringFieldOwned {
        StringFieldOwned::new(self.0.push("phase"), &self.1)
    }

    pub fn vertex_stage(self: &Self) -> GraphicsPipelineShaderStageOwned {
        GraphicsPipelineShaderStageOwned::new(self.0.push("vertex_stage"), &self.1)
    }
}
#[derive(Default)]
pub struct ShaderPackageAssetAccessor(PropertyPath);

impl FieldAccessor for ShaderPackageAssetAccessor {
    fn new(property_path: PropertyPath) -> Self {
        ShaderPackageAssetAccessor(property_path)
    }
}

impl RecordAccessor for ShaderPackageAssetAccessor {
    fn schema_name() -> &'static str {
        "ShaderPackageAsset"
    }
}

impl ShaderPackageAssetAccessor {
}
pub struct ShaderPackageAssetReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for ShaderPackageAssetReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        ShaderPackageAssetReader(property_path, data_container)
    }
}

impl<'a> RecordReader for ShaderPackageAssetReader<'a> {
    fn schema_name() -> &'static str {
        "ShaderPackageAsset"
    }
}

impl<'a> ShaderPackageAssetReader<'a> {
}
pub struct ShaderPackageAssetWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for ShaderPackageAssetWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        ShaderPackageAssetWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for ShaderPackageAssetWriter<'a> {
    fn schema_name() -> &'static str {
        "ShaderPackageAsset"
    }
}

impl<'a> ShaderPackageAssetWriter<'a> {
}
pub struct ShaderPackageAssetOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for ShaderPackageAssetOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        ShaderPackageAssetOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for ShaderPackageAssetOwned {
    fn schema_name() -> &'static str {
        "ShaderPackageAsset"
    }
}

impl ShaderPackageAssetOwned {
}
#[derive(Default)]
pub struct ShaderPackageImportedDataAccessor(PropertyPath);

impl FieldAccessor for ShaderPackageImportedDataAccessor {
    fn new(property_path: PropertyPath) -> Self {
        ShaderPackageImportedDataAccessor(property_path)
    }
}

impl RecordAccessor for ShaderPackageImportedDataAccessor {
    fn schema_name() -> &'static str {
        "ShaderPackageImportedData"
    }
}

impl ShaderPackageImportedDataAccessor {
    pub fn bytes(&self) -> BytesFieldAccessor {
        BytesFieldAccessor::new(self.0.push("bytes"))
    }
}
pub struct ShaderPackageImportedDataReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for ShaderPackageImportedDataReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        ShaderPackageImportedDataReader(property_path, data_container)
    }
}

impl<'a> RecordReader for ShaderPackageImportedDataReader<'a> {
    fn schema_name() -> &'static str {
        "ShaderPackageImportedData"
    }
}

impl<'a> ShaderPackageImportedDataReader<'a> {
    pub fn bytes(&self) -> BytesFieldReader {
        BytesFieldReader::new(self.0.push("bytes"), self.1)
    }
}
pub struct ShaderPackageImportedDataWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for ShaderPackageImportedDataWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        ShaderPackageImportedDataWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for ShaderPackageImportedDataWriter<'a> {
    fn schema_name() -> &'static str {
        "ShaderPackageImportedData"
    }
}

impl<'a> ShaderPackageImportedDataWriter<'a> {
    pub fn bytes(self: &'a Self) -> BytesFieldWriter {
        BytesFieldWriter::new(self.0.push("bytes"), &self.1)
    }
}
pub struct ShaderPackageImportedDataOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for ShaderPackageImportedDataOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        ShaderPackageImportedDataOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for ShaderPackageImportedDataOwned {
    fn schema_name() -> &'static str {
        "ShaderPackageImportedData"
    }
}

impl ShaderPackageImportedDataOwned {
    pub fn bytes(self: &Self) -> BytesFieldOwned {
        BytesFieldOwned::new(self.0.push("bytes"), &self.1)
    }
}
#[derive(Default)]
pub struct Vec3Accessor(PropertyPath);

impl FieldAccessor for Vec3Accessor {
    fn new(property_path: PropertyPath) -> Self {
        Vec3Accessor(property_path)
    }
}

impl RecordAccessor for Vec3Accessor {
    fn schema_name() -> &'static str {
        "Vec3"
    }
}

impl Vec3Accessor {
    pub fn x(&self) -> F32FieldAccessor {
        F32FieldAccessor::new(self.0.push("x"))
    }

    pub fn y(&self) -> F32FieldAccessor {
        F32FieldAccessor::new(self.0.push("y"))
    }

    pub fn z(&self) -> F32FieldAccessor {
        F32FieldAccessor::new(self.0.push("z"))
    }
}
pub struct Vec3Reader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for Vec3Reader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        Vec3Reader(property_path, data_container)
    }
}

impl<'a> RecordReader for Vec3Reader<'a> {
    fn schema_name() -> &'static str {
        "Vec3"
    }
}

impl<'a> Vec3Reader<'a> {
    pub fn x(&self) -> F32FieldReader {
        F32FieldReader::new(self.0.push("x"), self.1)
    }

    pub fn y(&self) -> F32FieldReader {
        F32FieldReader::new(self.0.push("y"), self.1)
    }

    pub fn z(&self) -> F32FieldReader {
        F32FieldReader::new(self.0.push("z"), self.1)
    }
}
pub struct Vec3Writer<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for Vec3Writer<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        Vec3Writer(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for Vec3Writer<'a> {
    fn schema_name() -> &'static str {
        "Vec3"
    }
}

impl<'a> Vec3Writer<'a> {
    pub fn x(self: &'a Self) -> F32FieldWriter {
        F32FieldWriter::new(self.0.push("x"), &self.1)
    }

    pub fn y(self: &'a Self) -> F32FieldWriter {
        F32FieldWriter::new(self.0.push("y"), &self.1)
    }

    pub fn z(self: &'a Self) -> F32FieldWriter {
        F32FieldWriter::new(self.0.push("z"), &self.1)
    }
}
pub struct Vec3Owned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for Vec3Owned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        Vec3Owned(property_path, data_container.clone())
    }
}

impl RecordOwned for Vec3Owned {
    fn schema_name() -> &'static str {
        "Vec3"
    }
}

impl Vec3Owned {
    pub fn x(self: &Self) -> F32FieldOwned {
        F32FieldOwned::new(self.0.push("x"), &self.1)
    }

    pub fn y(self: &Self) -> F32FieldOwned {
        F32FieldOwned::new(self.0.push("y"), &self.1)
    }

    pub fn z(self: &Self) -> F32FieldOwned {
        F32FieldOwned::new(self.0.push("z"), &self.1)
    }
}
#[derive(Default)]
pub struct Vec4Accessor(PropertyPath);

impl FieldAccessor for Vec4Accessor {
    fn new(property_path: PropertyPath) -> Self {
        Vec4Accessor(property_path)
    }
}

impl RecordAccessor for Vec4Accessor {
    fn schema_name() -> &'static str {
        "Vec4"
    }
}

impl Vec4Accessor {
    pub fn w(&self) -> F32FieldAccessor {
        F32FieldAccessor::new(self.0.push("w"))
    }

    pub fn x(&self) -> F32FieldAccessor {
        F32FieldAccessor::new(self.0.push("x"))
    }

    pub fn y(&self) -> F32FieldAccessor {
        F32FieldAccessor::new(self.0.push("y"))
    }

    pub fn z(&self) -> F32FieldAccessor {
        F32FieldAccessor::new(self.0.push("z"))
    }
}
pub struct Vec4Reader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for Vec4Reader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        Vec4Reader(property_path, data_container)
    }
}

impl<'a> RecordReader for Vec4Reader<'a> {
    fn schema_name() -> &'static str {
        "Vec4"
    }
}

impl<'a> Vec4Reader<'a> {
    pub fn w(&self) -> F32FieldReader {
        F32FieldReader::new(self.0.push("w"), self.1)
    }

    pub fn x(&self) -> F32FieldReader {
        F32FieldReader::new(self.0.push("x"), self.1)
    }

    pub fn y(&self) -> F32FieldReader {
        F32FieldReader::new(self.0.push("y"), self.1)
    }

    pub fn z(&self) -> F32FieldReader {
        F32FieldReader::new(self.0.push("z"), self.1)
    }
}
pub struct Vec4Writer<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for Vec4Writer<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        Vec4Writer(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for Vec4Writer<'a> {
    fn schema_name() -> &'static str {
        "Vec4"
    }
}

impl<'a> Vec4Writer<'a> {
    pub fn w(self: &'a Self) -> F32FieldWriter {
        F32FieldWriter::new(self.0.push("w"), &self.1)
    }

    pub fn x(self: &'a Self) -> F32FieldWriter {
        F32FieldWriter::new(self.0.push("x"), &self.1)
    }

    pub fn y(self: &'a Self) -> F32FieldWriter {
        F32FieldWriter::new(self.0.push("y"), &self.1)
    }

    pub fn z(self: &'a Self) -> F32FieldWriter {
        F32FieldWriter::new(self.0.push("z"), &self.1)
    }
}
pub struct Vec4Owned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for Vec4Owned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        Vec4Owned(property_path, data_container.clone())
    }
}

impl RecordOwned for Vec4Owned {
    fn schema_name() -> &'static str {
        "Vec4"
    }
}

impl Vec4Owned {
    pub fn w(self: &Self) -> F32FieldOwned {
        F32FieldOwned::new(self.0.push("w"), &self.1)
    }

    pub fn x(self: &Self) -> F32FieldOwned {
        F32FieldOwned::new(self.0.push("x"), &self.1)
    }

    pub fn y(self: &Self) -> F32FieldOwned {
        F32FieldOwned::new(self.0.push("y"), &self.1)
    }

    pub fn z(self: &Self) -> F32FieldOwned {
        F32FieldOwned::new(self.0.push("z"), &self.1)
    }
}
