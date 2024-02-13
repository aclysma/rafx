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
pub struct ComputePipelineAssetRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for ComputePipelineAssetRef<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: DataContainerRef<'a>,
    ) -> Self {
        ComputePipelineAssetRef(property_path, data_container)
    }
}

impl<'a> RecordRef for ComputePipelineAssetRef<'a> {
    fn schema_name() -> &'static str {
        "ComputePipelineAsset"
    }
}

impl<'a> ComputePipelineAssetRef<'a> {
    pub fn entry_name(&self) -> StringFieldRef {
        StringFieldRef::new(self.0.push("entry_name"), self.1.clone())
    }

    pub fn shader_module(&self) -> AssetRefFieldRef {
        AssetRefFieldRef::new(self.0.push("shader_module"), self.1.clone())
    }
}
pub struct ComputePipelineAssetRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for ComputePipelineAssetRefMut<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<DataContainerRefMut<'a>>>,
    ) -> Self {
        ComputePipelineAssetRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for ComputePipelineAssetRefMut<'a> {
    fn schema_name() -> &'static str {
        "ComputePipelineAsset"
    }
}

impl<'a> ComputePipelineAssetRefMut<'a> {
    pub fn entry_name(self: &'a Self) -> StringFieldRefMut {
        StringFieldRefMut::new(self.0.push("entry_name"), &self.1)
    }

    pub fn shader_module(self: &'a Self) -> AssetRefFieldRefMut {
        AssetRefFieldRefMut::new(self.0.push("shader_module"), &self.1)
    }
}
pub struct ComputePipelineAssetRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for ComputePipelineAssetRecord {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<Option<DataContainer>>>,
    ) -> Self {
        ComputePipelineAssetRecord(property_path, data_container.clone())
    }
}

impl Record for ComputePipelineAssetRecord {
    type Reader<'a> = ComputePipelineAssetRef<'a>;
    type Writer<'a> = ComputePipelineAssetRefMut<'a>;
    type Accessor = ComputePipelineAssetAccessor;

    fn schema_name() -> &'static str {
        "ComputePipelineAsset"
    }
}

impl ComputePipelineAssetRecord {
    pub fn entry_name(self: &Self) -> StringField {
        StringField::new(self.0.push("entry_name"), &self.1)
    }

    pub fn shader_module(self: &Self) -> AssetRefField {
        AssetRefField::new(self.0.push("shader_module"), &self.1)
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

impl GpuCompressedImageAssetAccessor {}
pub struct GpuCompressedImageAssetRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for GpuCompressedImageAssetRef<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: DataContainerRef<'a>,
    ) -> Self {
        GpuCompressedImageAssetRef(property_path, data_container)
    }
}

impl<'a> RecordRef for GpuCompressedImageAssetRef<'a> {
    fn schema_name() -> &'static str {
        "GpuCompressedImageAsset"
    }
}

impl<'a> GpuCompressedImageAssetRef<'a> {}
pub struct GpuCompressedImageAssetRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for GpuCompressedImageAssetRefMut<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<DataContainerRefMut<'a>>>,
    ) -> Self {
        GpuCompressedImageAssetRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for GpuCompressedImageAssetRefMut<'a> {
    fn schema_name() -> &'static str {
        "GpuCompressedImageAsset"
    }
}

impl<'a> GpuCompressedImageAssetRefMut<'a> {}
pub struct GpuCompressedImageAssetRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for GpuCompressedImageAssetRecord {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<Option<DataContainer>>>,
    ) -> Self {
        GpuCompressedImageAssetRecord(property_path, data_container.clone())
    }
}

impl Record for GpuCompressedImageAssetRecord {
    type Reader<'a> = GpuCompressedImageAssetRef<'a>;
    type Writer<'a> = GpuCompressedImageAssetRefMut<'a>;
    type Accessor = GpuCompressedImageAssetAccessor;

    fn schema_name() -> &'static str {
        "GpuCompressedImageAsset"
    }
}

impl GpuCompressedImageAssetRecord {}
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
    pub fn data_layers(&self) -> DynamicArrayFieldAccessor<GpuImageSubresourceLayerAccessor> {
        DynamicArrayFieldAccessor::<GpuImageSubresourceLayerAccessor>::new(
            self.0.push("data_layers"),
        )
    }

    pub fn data_single_buffer(&self) -> BytesFieldAccessor {
        BytesFieldAccessor::new(self.0.push("data_single_buffer"))
    }

    pub fn format(&self) -> EnumFieldAccessor<GpuImageAssetDataFormatEnum> {
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
pub struct GpuCompressedImageImportedDataRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for GpuCompressedImageImportedDataRef<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: DataContainerRef<'a>,
    ) -> Self {
        GpuCompressedImageImportedDataRef(property_path, data_container)
    }
}

impl<'a> RecordRef for GpuCompressedImageImportedDataRef<'a> {
    fn schema_name() -> &'static str {
        "GpuCompressedImageImportedData"
    }
}

impl<'a> GpuCompressedImageImportedDataRef<'a> {
    pub fn data_layers(&self) -> DynamicArrayFieldRef<GpuImageSubresourceLayerRef> {
        DynamicArrayFieldRef::<GpuImageSubresourceLayerRef>::new(
            self.0.push("data_layers"),
            self.1.clone(),
        )
    }

    pub fn data_single_buffer(&self) -> BytesFieldRef {
        BytesFieldRef::new(self.0.push("data_single_buffer"), self.1.clone())
    }

    pub fn format(&self) -> EnumFieldRef<GpuImageAssetDataFormatEnum> {
        EnumFieldRef::<GpuImageAssetDataFormatEnum>::new(self.0.push("format"), self.1.clone())
    }

    pub fn height(&self) -> U32FieldRef {
        U32FieldRef::new(self.0.push("height"), self.1.clone())
    }

    pub fn is_cube_texture(&self) -> BooleanFieldRef {
        BooleanFieldRef::new(self.0.push("is_cube_texture"), self.1.clone())
    }

    pub fn width(&self) -> U32FieldRef {
        U32FieldRef::new(self.0.push("width"), self.1.clone())
    }
}
pub struct GpuCompressedImageImportedDataRefMut<'a>(
    PropertyPath,
    Rc<RefCell<DataContainerRefMut<'a>>>,
);

impl<'a> FieldRefMut<'a> for GpuCompressedImageImportedDataRefMut<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<DataContainerRefMut<'a>>>,
    ) -> Self {
        GpuCompressedImageImportedDataRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for GpuCompressedImageImportedDataRefMut<'a> {
    fn schema_name() -> &'static str {
        "GpuCompressedImageImportedData"
    }
}

impl<'a> GpuCompressedImageImportedDataRefMut<'a> {
    pub fn data_layers(self: &'a Self) -> DynamicArrayFieldRefMut<GpuImageSubresourceLayerRefMut> {
        DynamicArrayFieldRefMut::<GpuImageSubresourceLayerRefMut>::new(
            self.0.push("data_layers"),
            &self.1,
        )
    }

    pub fn data_single_buffer(self: &'a Self) -> BytesFieldRefMut {
        BytesFieldRefMut::new(self.0.push("data_single_buffer"), &self.1)
    }

    pub fn format(self: &'a Self) -> EnumFieldRefMut<GpuImageAssetDataFormatEnum> {
        EnumFieldRefMut::<GpuImageAssetDataFormatEnum>::new(self.0.push("format"), &self.1)
    }

    pub fn height(self: &'a Self) -> U32FieldRefMut {
        U32FieldRefMut::new(self.0.push("height"), &self.1)
    }

    pub fn is_cube_texture(self: &'a Self) -> BooleanFieldRefMut {
        BooleanFieldRefMut::new(self.0.push("is_cube_texture"), &self.1)
    }

    pub fn width(self: &'a Self) -> U32FieldRefMut {
        U32FieldRefMut::new(self.0.push("width"), &self.1)
    }
}
pub struct GpuCompressedImageImportedDataRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for GpuCompressedImageImportedDataRecord {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<Option<DataContainer>>>,
    ) -> Self {
        GpuCompressedImageImportedDataRecord(property_path, data_container.clone())
    }
}

impl Record for GpuCompressedImageImportedDataRecord {
    type Reader<'a> = GpuCompressedImageImportedDataRef<'a>;
    type Writer<'a> = GpuCompressedImageImportedDataRefMut<'a>;
    type Accessor = GpuCompressedImageImportedDataAccessor;

    fn schema_name() -> &'static str {
        "GpuCompressedImageImportedData"
    }
}

impl GpuCompressedImageImportedDataRecord {
    pub fn data_layers(self: &Self) -> DynamicArrayField<GpuImageSubresourceLayerRecord> {
        DynamicArrayField::<GpuImageSubresourceLayerRecord>::new(
            self.0.push("data_layers"),
            &self.1,
        )
    }

    pub fn data_single_buffer(self: &Self) -> BytesField {
        BytesField::new(self.0.push("data_single_buffer"), &self.1)
    }

    pub fn format(self: &Self) -> EnumField<GpuImageAssetDataFormatEnum> {
        EnumField::<GpuImageAssetDataFormatEnum>::new(self.0.push("format"), &self.1)
    }

    pub fn height(self: &Self) -> U32Field {
        U32Field::new(self.0.push("height"), &self.1)
    }

    pub fn is_cube_texture(self: &Self) -> BooleanField {
        BooleanField::new(self.0.push("is_cube_texture"), &self.1)
    }

    pub fn width(self: &Self) -> U32Field {
        U32Field::new(self.0.push("width"), &self.1)
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

    pub fn color_space(&self) -> EnumFieldAccessor<GpuImageColorSpaceEnum> {
        EnumFieldAccessor::<GpuImageColorSpaceEnum>::new(self.0.push("color_space"))
    }

    pub fn mip_generation(&self) -> EnumFieldAccessor<GpuImageMipGenerationEnum> {
        EnumFieldAccessor::<GpuImageMipGenerationEnum>::new(self.0.push("mip_generation"))
    }
}
pub struct GpuImageAssetRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for GpuImageAssetRef<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: DataContainerRef<'a>,
    ) -> Self {
        GpuImageAssetRef(property_path, data_container)
    }
}

impl<'a> RecordRef for GpuImageAssetRef<'a> {
    fn schema_name() -> &'static str {
        "GpuImageAsset"
    }
}

impl<'a> GpuImageAssetRef<'a> {
    pub fn basis_compression(&self) -> BooleanFieldRef {
        BooleanFieldRef::new(self.0.push("basis_compression"), self.1.clone())
    }

    pub fn basis_compression_settings(&self) -> GpuImageBasisCompressionSettingsRef {
        GpuImageBasisCompressionSettingsRef::new(
            self.0.push("basis_compression_settings"),
            self.1.clone(),
        )
    }

    pub fn color_space(&self) -> EnumFieldRef<GpuImageColorSpaceEnum> {
        EnumFieldRef::<GpuImageColorSpaceEnum>::new(self.0.push("color_space"), self.1.clone())
    }

    pub fn mip_generation(&self) -> EnumFieldRef<GpuImageMipGenerationEnum> {
        EnumFieldRef::<GpuImageMipGenerationEnum>::new(
            self.0.push("mip_generation"),
            self.1.clone(),
        )
    }
}
pub struct GpuImageAssetRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for GpuImageAssetRefMut<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<DataContainerRefMut<'a>>>,
    ) -> Self {
        GpuImageAssetRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for GpuImageAssetRefMut<'a> {
    fn schema_name() -> &'static str {
        "GpuImageAsset"
    }
}

impl<'a> GpuImageAssetRefMut<'a> {
    pub fn basis_compression(self: &'a Self) -> BooleanFieldRefMut {
        BooleanFieldRefMut::new(self.0.push("basis_compression"), &self.1)
    }

    pub fn basis_compression_settings(self: &'a Self) -> GpuImageBasisCompressionSettingsRefMut {
        GpuImageBasisCompressionSettingsRefMut::new(
            self.0.push("basis_compression_settings"),
            &self.1,
        )
    }

    pub fn color_space(self: &'a Self) -> EnumFieldRefMut<GpuImageColorSpaceEnum> {
        EnumFieldRefMut::<GpuImageColorSpaceEnum>::new(self.0.push("color_space"), &self.1)
    }

    pub fn mip_generation(self: &'a Self) -> EnumFieldRefMut<GpuImageMipGenerationEnum> {
        EnumFieldRefMut::<GpuImageMipGenerationEnum>::new(self.0.push("mip_generation"), &self.1)
    }
}
pub struct GpuImageAssetRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for GpuImageAssetRecord {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<Option<DataContainer>>>,
    ) -> Self {
        GpuImageAssetRecord(property_path, data_container.clone())
    }
}

impl Record for GpuImageAssetRecord {
    type Reader<'a> = GpuImageAssetRef<'a>;
    type Writer<'a> = GpuImageAssetRefMut<'a>;
    type Accessor = GpuImageAssetAccessor;

    fn schema_name() -> &'static str {
        "GpuImageAsset"
    }
}

impl GpuImageAssetRecord {
    pub fn basis_compression(self: &Self) -> BooleanField {
        BooleanField::new(self.0.push("basis_compression"), &self.1)
    }

    pub fn basis_compression_settings(self: &Self) -> GpuImageBasisCompressionSettingsRecord {
        GpuImageBasisCompressionSettingsRecord::new(
            self.0.push("basis_compression_settings"),
            &self.1,
        )
    }

    pub fn color_space(self: &Self) -> EnumField<GpuImageColorSpaceEnum> {
        EnumField::<GpuImageColorSpaceEnum>::new(self.0.push("color_space"), &self.1)
    }

    pub fn mip_generation(self: &Self) -> EnumField<GpuImageMipGenerationEnum> {
        EnumField::<GpuImageMipGenerationEnum>::new(self.0.push("mip_generation"), &self.1)
    }
}
#[derive(Copy, Clone)]
#[allow(non_camel_case_types)]
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
    pub fn compression_type(&self) -> EnumFieldAccessor<GpuImageBasisCompressionTypeEnum> {
        EnumFieldAccessor::<GpuImageBasisCompressionTypeEnum>::new(self.0.push("compression_type"))
    }

    pub fn quality(&self) -> U32FieldAccessor {
        U32FieldAccessor::new(self.0.push("quality"))
    }
}
pub struct GpuImageBasisCompressionSettingsRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for GpuImageBasisCompressionSettingsRef<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: DataContainerRef<'a>,
    ) -> Self {
        GpuImageBasisCompressionSettingsRef(property_path, data_container)
    }
}

impl<'a> RecordRef for GpuImageBasisCompressionSettingsRef<'a> {
    fn schema_name() -> &'static str {
        "GpuImageBasisCompressionSettings"
    }
}

impl<'a> GpuImageBasisCompressionSettingsRef<'a> {
    pub fn compression_type(&self) -> EnumFieldRef<GpuImageBasisCompressionTypeEnum> {
        EnumFieldRef::<GpuImageBasisCompressionTypeEnum>::new(
            self.0.push("compression_type"),
            self.1.clone(),
        )
    }

    pub fn quality(&self) -> U32FieldRef {
        U32FieldRef::new(self.0.push("quality"), self.1.clone())
    }
}
pub struct GpuImageBasisCompressionSettingsRefMut<'a>(
    PropertyPath,
    Rc<RefCell<DataContainerRefMut<'a>>>,
);

impl<'a> FieldRefMut<'a> for GpuImageBasisCompressionSettingsRefMut<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<DataContainerRefMut<'a>>>,
    ) -> Self {
        GpuImageBasisCompressionSettingsRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for GpuImageBasisCompressionSettingsRefMut<'a> {
    fn schema_name() -> &'static str {
        "GpuImageBasisCompressionSettings"
    }
}

impl<'a> GpuImageBasisCompressionSettingsRefMut<'a> {
    pub fn compression_type(self: &'a Self) -> EnumFieldRefMut<GpuImageBasisCompressionTypeEnum> {
        EnumFieldRefMut::<GpuImageBasisCompressionTypeEnum>::new(
            self.0.push("compression_type"),
            &self.1,
        )
    }

    pub fn quality(self: &'a Self) -> U32FieldRefMut {
        U32FieldRefMut::new(self.0.push("quality"), &self.1)
    }
}
pub struct GpuImageBasisCompressionSettingsRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for GpuImageBasisCompressionSettingsRecord {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<Option<DataContainer>>>,
    ) -> Self {
        GpuImageBasisCompressionSettingsRecord(property_path, data_container.clone())
    }
}

impl Record for GpuImageBasisCompressionSettingsRecord {
    type Reader<'a> = GpuImageBasisCompressionSettingsRef<'a>;
    type Writer<'a> = GpuImageBasisCompressionSettingsRefMut<'a>;
    type Accessor = GpuImageBasisCompressionSettingsAccessor;

    fn schema_name() -> &'static str {
        "GpuImageBasisCompressionSettings"
    }
}

impl GpuImageBasisCompressionSettingsRecord {
    pub fn compression_type(self: &Self) -> EnumField<GpuImageBasisCompressionTypeEnum> {
        EnumField::<GpuImageBasisCompressionTypeEnum>::new(self.0.push("compression_type"), &self.1)
    }

    pub fn quality(self: &Self) -> U32Field {
        U32Field::new(self.0.push("quality"), &self.1)
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
pub struct GpuImageImportedDataRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for GpuImageImportedDataRef<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: DataContainerRef<'a>,
    ) -> Self {
        GpuImageImportedDataRef(property_path, data_container)
    }
}

impl<'a> RecordRef for GpuImageImportedDataRef<'a> {
    fn schema_name() -> &'static str {
        "GpuImageImportedData"
    }
}

impl<'a> GpuImageImportedDataRef<'a> {
    pub fn height(&self) -> U32FieldRef {
        U32FieldRef::new(self.0.push("height"), self.1.clone())
    }

    pub fn image_bytes(&self) -> BytesFieldRef {
        BytesFieldRef::new(self.0.push("image_bytes"), self.1.clone())
    }

    pub fn width(&self) -> U32FieldRef {
        U32FieldRef::new(self.0.push("width"), self.1.clone())
    }
}
pub struct GpuImageImportedDataRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for GpuImageImportedDataRefMut<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<DataContainerRefMut<'a>>>,
    ) -> Self {
        GpuImageImportedDataRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for GpuImageImportedDataRefMut<'a> {
    fn schema_name() -> &'static str {
        "GpuImageImportedData"
    }
}

impl<'a> GpuImageImportedDataRefMut<'a> {
    pub fn height(self: &'a Self) -> U32FieldRefMut {
        U32FieldRefMut::new(self.0.push("height"), &self.1)
    }

    pub fn image_bytes(self: &'a Self) -> BytesFieldRefMut {
        BytesFieldRefMut::new(self.0.push("image_bytes"), &self.1)
    }

    pub fn width(self: &'a Self) -> U32FieldRefMut {
        U32FieldRefMut::new(self.0.push("width"), &self.1)
    }
}
pub struct GpuImageImportedDataRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for GpuImageImportedDataRecord {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<Option<DataContainer>>>,
    ) -> Self {
        GpuImageImportedDataRecord(property_path, data_container.clone())
    }
}

impl Record for GpuImageImportedDataRecord {
    type Reader<'a> = GpuImageImportedDataRef<'a>;
    type Writer<'a> = GpuImageImportedDataRefMut<'a>;
    type Accessor = GpuImageImportedDataAccessor;

    fn schema_name() -> &'static str {
        "GpuImageImportedData"
    }
}

impl GpuImageImportedDataRecord {
    pub fn height(self: &Self) -> U32Field {
        U32Field::new(self.0.push("height"), &self.1)
    }

    pub fn image_bytes(self: &Self) -> BytesField {
        BytesField::new(self.0.push("image_bytes"), &self.1)
    }

    pub fn width(self: &Self) -> U32Field {
        U32Field::new(self.0.push("width"), &self.1)
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
    pub fn mip_levels(&self) -> DynamicArrayFieldAccessor<GpuImageSubresourceMipLevelAccessor> {
        DynamicArrayFieldAccessor::<GpuImageSubresourceMipLevelAccessor>::new(
            self.0.push("mip_levels"),
        )
    }
}
pub struct GpuImageSubresourceLayerRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for GpuImageSubresourceLayerRef<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: DataContainerRef<'a>,
    ) -> Self {
        GpuImageSubresourceLayerRef(property_path, data_container)
    }
}

impl<'a> RecordRef for GpuImageSubresourceLayerRef<'a> {
    fn schema_name() -> &'static str {
        "GpuImageSubresourceLayer"
    }
}

impl<'a> GpuImageSubresourceLayerRef<'a> {
    pub fn mip_levels(&self) -> DynamicArrayFieldRef<GpuImageSubresourceMipLevelRef> {
        DynamicArrayFieldRef::<GpuImageSubresourceMipLevelRef>::new(
            self.0.push("mip_levels"),
            self.1.clone(),
        )
    }
}
pub struct GpuImageSubresourceLayerRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for GpuImageSubresourceLayerRefMut<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<DataContainerRefMut<'a>>>,
    ) -> Self {
        GpuImageSubresourceLayerRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for GpuImageSubresourceLayerRefMut<'a> {
    fn schema_name() -> &'static str {
        "GpuImageSubresourceLayer"
    }
}

impl<'a> GpuImageSubresourceLayerRefMut<'a> {
    pub fn mip_levels(
        self: &'a Self
    ) -> DynamicArrayFieldRefMut<GpuImageSubresourceMipLevelRefMut> {
        DynamicArrayFieldRefMut::<GpuImageSubresourceMipLevelRefMut>::new(
            self.0.push("mip_levels"),
            &self.1,
        )
    }
}
pub struct GpuImageSubresourceLayerRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for GpuImageSubresourceLayerRecord {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<Option<DataContainer>>>,
    ) -> Self {
        GpuImageSubresourceLayerRecord(property_path, data_container.clone())
    }
}

impl Record for GpuImageSubresourceLayerRecord {
    type Reader<'a> = GpuImageSubresourceLayerRef<'a>;
    type Writer<'a> = GpuImageSubresourceLayerRefMut<'a>;
    type Accessor = GpuImageSubresourceLayerAccessor;

    fn schema_name() -> &'static str {
        "GpuImageSubresourceLayer"
    }
}

impl GpuImageSubresourceLayerRecord {
    pub fn mip_levels(self: &Self) -> DynamicArrayField<GpuImageSubresourceMipLevelRecord> {
        DynamicArrayField::<GpuImageSubresourceMipLevelRecord>::new(
            self.0.push("mip_levels"),
            &self.1,
        )
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
pub struct GpuImageSubresourceMipLevelRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for GpuImageSubresourceMipLevelRef<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: DataContainerRef<'a>,
    ) -> Self {
        GpuImageSubresourceMipLevelRef(property_path, data_container)
    }
}

impl<'a> RecordRef for GpuImageSubresourceMipLevelRef<'a> {
    fn schema_name() -> &'static str {
        "GpuImageSubresourceMipLevel"
    }
}

impl<'a> GpuImageSubresourceMipLevelRef<'a> {
    pub fn bytes(&self) -> BytesFieldRef {
        BytesFieldRef::new(self.0.push("bytes"), self.1.clone())
    }

    pub fn height(&self) -> U32FieldRef {
        U32FieldRef::new(self.0.push("height"), self.1.clone())
    }

    pub fn width(&self) -> U32FieldRef {
        U32FieldRef::new(self.0.push("width"), self.1.clone())
    }
}
pub struct GpuImageSubresourceMipLevelRefMut<'a>(
    PropertyPath,
    Rc<RefCell<DataContainerRefMut<'a>>>,
);

impl<'a> FieldRefMut<'a> for GpuImageSubresourceMipLevelRefMut<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<DataContainerRefMut<'a>>>,
    ) -> Self {
        GpuImageSubresourceMipLevelRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for GpuImageSubresourceMipLevelRefMut<'a> {
    fn schema_name() -> &'static str {
        "GpuImageSubresourceMipLevel"
    }
}

impl<'a> GpuImageSubresourceMipLevelRefMut<'a> {
    pub fn bytes(self: &'a Self) -> BytesFieldRefMut {
        BytesFieldRefMut::new(self.0.push("bytes"), &self.1)
    }

    pub fn height(self: &'a Self) -> U32FieldRefMut {
        U32FieldRefMut::new(self.0.push("height"), &self.1)
    }

    pub fn width(self: &'a Self) -> U32FieldRefMut {
        U32FieldRefMut::new(self.0.push("width"), &self.1)
    }
}
pub struct GpuImageSubresourceMipLevelRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for GpuImageSubresourceMipLevelRecord {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<Option<DataContainer>>>,
    ) -> Self {
        GpuImageSubresourceMipLevelRecord(property_path, data_container.clone())
    }
}

impl Record for GpuImageSubresourceMipLevelRecord {
    type Reader<'a> = GpuImageSubresourceMipLevelRef<'a>;
    type Writer<'a> = GpuImageSubresourceMipLevelRefMut<'a>;
    type Accessor = GpuImageSubresourceMipLevelAccessor;

    fn schema_name() -> &'static str {
        "GpuImageSubresourceMipLevel"
    }
}

impl GpuImageSubresourceMipLevelRecord {
    pub fn bytes(self: &Self) -> BytesField {
        BytesField::new(self.0.push("bytes"), &self.1)
    }

    pub fn height(self: &Self) -> U32Field {
        U32Field::new(self.0.push("height"), &self.1)
    }

    pub fn width(self: &Self) -> U32Field {
        U32Field::new(self.0.push("width"), &self.1)
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
pub struct GraphicsPipelineShaderStageRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for GraphicsPipelineShaderStageRef<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: DataContainerRef<'a>,
    ) -> Self {
        GraphicsPipelineShaderStageRef(property_path, data_container)
    }
}

impl<'a> RecordRef for GraphicsPipelineShaderStageRef<'a> {
    fn schema_name() -> &'static str {
        "GraphicsPipelineShaderStage"
    }
}

impl<'a> GraphicsPipelineShaderStageRef<'a> {
    pub fn entry_name(&self) -> StringFieldRef {
        StringFieldRef::new(self.0.push("entry_name"), self.1.clone())
    }

    pub fn shader_module(&self) -> AssetRefFieldRef {
        AssetRefFieldRef::new(self.0.push("shader_module"), self.1.clone())
    }
}
pub struct GraphicsPipelineShaderStageRefMut<'a>(
    PropertyPath,
    Rc<RefCell<DataContainerRefMut<'a>>>,
);

impl<'a> FieldRefMut<'a> for GraphicsPipelineShaderStageRefMut<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<DataContainerRefMut<'a>>>,
    ) -> Self {
        GraphicsPipelineShaderStageRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for GraphicsPipelineShaderStageRefMut<'a> {
    fn schema_name() -> &'static str {
        "GraphicsPipelineShaderStage"
    }
}

impl<'a> GraphicsPipelineShaderStageRefMut<'a> {
    pub fn entry_name(self: &'a Self) -> StringFieldRefMut {
        StringFieldRefMut::new(self.0.push("entry_name"), &self.1)
    }

    pub fn shader_module(self: &'a Self) -> AssetRefFieldRefMut {
        AssetRefFieldRefMut::new(self.0.push("shader_module"), &self.1)
    }
}
pub struct GraphicsPipelineShaderStageRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for GraphicsPipelineShaderStageRecord {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<Option<DataContainer>>>,
    ) -> Self {
        GraphicsPipelineShaderStageRecord(property_path, data_container.clone())
    }
}

impl Record for GraphicsPipelineShaderStageRecord {
    type Reader<'a> = GraphicsPipelineShaderStageRef<'a>;
    type Writer<'a> = GraphicsPipelineShaderStageRefMut<'a>;
    type Accessor = GraphicsPipelineShaderStageAccessor;

    fn schema_name() -> &'static str {
        "GraphicsPipelineShaderStage"
    }
}

impl GraphicsPipelineShaderStageRecord {
    pub fn entry_name(self: &Self) -> StringField {
        StringField::new(self.0.push("entry_name"), &self.1)
    }

    pub fn shader_module(self: &Self) -> AssetRefField {
        AssetRefField::new(self.0.push("shader_module"), &self.1)
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
    pub fn passes(&self) -> DynamicArrayFieldAccessor<MaterialPassAccessor> {
        DynamicArrayFieldAccessor::<MaterialPassAccessor>::new(self.0.push("passes"))
    }
}
pub struct MaterialAssetRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for MaterialAssetRef<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: DataContainerRef<'a>,
    ) -> Self {
        MaterialAssetRef(property_path, data_container)
    }
}

impl<'a> RecordRef for MaterialAssetRef<'a> {
    fn schema_name() -> &'static str {
        "MaterialAsset"
    }
}

impl<'a> MaterialAssetRef<'a> {
    pub fn passes(&self) -> DynamicArrayFieldRef<MaterialPassRef> {
        DynamicArrayFieldRef::<MaterialPassRef>::new(self.0.push("passes"), self.1.clone())
    }
}
pub struct MaterialAssetRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for MaterialAssetRefMut<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<DataContainerRefMut<'a>>>,
    ) -> Self {
        MaterialAssetRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for MaterialAssetRefMut<'a> {
    fn schema_name() -> &'static str {
        "MaterialAsset"
    }
}

impl<'a> MaterialAssetRefMut<'a> {
    pub fn passes(self: &'a Self) -> DynamicArrayFieldRefMut<MaterialPassRefMut> {
        DynamicArrayFieldRefMut::<MaterialPassRefMut>::new(self.0.push("passes"), &self.1)
    }
}
pub struct MaterialAssetRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for MaterialAssetRecord {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<Option<DataContainer>>>,
    ) -> Self {
        MaterialAssetRecord(property_path, data_container.clone())
    }
}

impl Record for MaterialAssetRecord {
    type Reader<'a> = MaterialAssetRef<'a>;
    type Writer<'a> = MaterialAssetRefMut<'a>;
    type Accessor = MaterialAssetAccessor;

    fn schema_name() -> &'static str {
        "MaterialAsset"
    }
}

impl MaterialAssetRecord {
    pub fn passes(self: &Self) -> DynamicArrayField<MaterialPassRecord> {
        DynamicArrayField::<MaterialPassRecord>::new(self.0.push("passes"), &self.1)
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

    pub fn slot_assignments(
        &self
    ) -> DynamicArrayFieldAccessor<MaterialInstanceSlotAssignmentAccessor> {
        DynamicArrayFieldAccessor::<MaterialInstanceSlotAssignmentAccessor>::new(
            self.0.push("slot_assignments"),
        )
    }
}
pub struct MaterialInstanceAssetRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for MaterialInstanceAssetRef<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: DataContainerRef<'a>,
    ) -> Self {
        MaterialInstanceAssetRef(property_path, data_container)
    }
}

impl<'a> RecordRef for MaterialInstanceAssetRef<'a> {
    fn schema_name() -> &'static str {
        "MaterialInstanceAsset"
    }
}

impl<'a> MaterialInstanceAssetRef<'a> {
    pub fn material(&self) -> AssetRefFieldRef {
        AssetRefFieldRef::new(self.0.push("material"), self.1.clone())
    }

    pub fn slot_assignments(&self) -> DynamicArrayFieldRef<MaterialInstanceSlotAssignmentRef> {
        DynamicArrayFieldRef::<MaterialInstanceSlotAssignmentRef>::new(
            self.0.push("slot_assignments"),
            self.1.clone(),
        )
    }
}
pub struct MaterialInstanceAssetRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for MaterialInstanceAssetRefMut<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<DataContainerRefMut<'a>>>,
    ) -> Self {
        MaterialInstanceAssetRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for MaterialInstanceAssetRefMut<'a> {
    fn schema_name() -> &'static str {
        "MaterialInstanceAsset"
    }
}

impl<'a> MaterialInstanceAssetRefMut<'a> {
    pub fn material(self: &'a Self) -> AssetRefFieldRefMut {
        AssetRefFieldRefMut::new(self.0.push("material"), &self.1)
    }

    pub fn slot_assignments(
        self: &'a Self
    ) -> DynamicArrayFieldRefMut<MaterialInstanceSlotAssignmentRefMut> {
        DynamicArrayFieldRefMut::<MaterialInstanceSlotAssignmentRefMut>::new(
            self.0.push("slot_assignments"),
            &self.1,
        )
    }
}
pub struct MaterialInstanceAssetRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for MaterialInstanceAssetRecord {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<Option<DataContainer>>>,
    ) -> Self {
        MaterialInstanceAssetRecord(property_path, data_container.clone())
    }
}

impl Record for MaterialInstanceAssetRecord {
    type Reader<'a> = MaterialInstanceAssetRef<'a>;
    type Writer<'a> = MaterialInstanceAssetRefMut<'a>;
    type Accessor = MaterialInstanceAssetAccessor;

    fn schema_name() -> &'static str {
        "MaterialInstanceAsset"
    }
}

impl MaterialInstanceAssetRecord {
    pub fn material(self: &Self) -> AssetRefField {
        AssetRefField::new(self.0.push("material"), &self.1)
    }

    pub fn slot_assignments(
        self: &Self
    ) -> DynamicArrayField<MaterialInstanceSlotAssignmentRecord> {
        DynamicArrayField::<MaterialInstanceSlotAssignmentRecord>::new(
            self.0.push("slot_assignments"),
            &self.1,
        )
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

    pub fn buffer_data(&self) -> NullableFieldAccessor<BytesFieldAccessor> {
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
pub struct MaterialInstanceSlotAssignmentRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for MaterialInstanceSlotAssignmentRef<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: DataContainerRef<'a>,
    ) -> Self {
        MaterialInstanceSlotAssignmentRef(property_path, data_container)
    }
}

impl<'a> RecordRef for MaterialInstanceSlotAssignmentRef<'a> {
    fn schema_name() -> &'static str {
        "MaterialInstanceSlotAssignment"
    }
}

impl<'a> MaterialInstanceSlotAssignmentRef<'a> {
    pub fn array_index(&self) -> U32FieldRef {
        U32FieldRef::new(self.0.push("array_index"), self.1.clone())
    }

    pub fn buffer_data(&self) -> NullableFieldRef<BytesFieldRef> {
        NullableFieldRef::<BytesFieldRef>::new(self.0.push("buffer_data"), self.1.clone())
    }

    pub fn image(&self) -> AssetRefFieldRef {
        AssetRefFieldRef::new(self.0.push("image"), self.1.clone())
    }

    pub fn sampler(&self) -> StringFieldRef {
        StringFieldRef::new(self.0.push("sampler"), self.1.clone())
    }

    pub fn slot_name(&self) -> StringFieldRef {
        StringFieldRef::new(self.0.push("slot_name"), self.1.clone())
    }
}
pub struct MaterialInstanceSlotAssignmentRefMut<'a>(
    PropertyPath,
    Rc<RefCell<DataContainerRefMut<'a>>>,
);

impl<'a> FieldRefMut<'a> for MaterialInstanceSlotAssignmentRefMut<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<DataContainerRefMut<'a>>>,
    ) -> Self {
        MaterialInstanceSlotAssignmentRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for MaterialInstanceSlotAssignmentRefMut<'a> {
    fn schema_name() -> &'static str {
        "MaterialInstanceSlotAssignment"
    }
}

impl<'a> MaterialInstanceSlotAssignmentRefMut<'a> {
    pub fn array_index(self: &'a Self) -> U32FieldRefMut {
        U32FieldRefMut::new(self.0.push("array_index"), &self.1)
    }

    pub fn buffer_data(self: &'a Self) -> NullableFieldRefMut<BytesFieldRefMut> {
        NullableFieldRefMut::<BytesFieldRefMut>::new(self.0.push("buffer_data"), &self.1)
    }

    pub fn image(self: &'a Self) -> AssetRefFieldRefMut {
        AssetRefFieldRefMut::new(self.0.push("image"), &self.1)
    }

    pub fn sampler(self: &'a Self) -> StringFieldRefMut {
        StringFieldRefMut::new(self.0.push("sampler"), &self.1)
    }

    pub fn slot_name(self: &'a Self) -> StringFieldRefMut {
        StringFieldRefMut::new(self.0.push("slot_name"), &self.1)
    }
}
pub struct MaterialInstanceSlotAssignmentRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for MaterialInstanceSlotAssignmentRecord {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<Option<DataContainer>>>,
    ) -> Self {
        MaterialInstanceSlotAssignmentRecord(property_path, data_container.clone())
    }
}

impl Record for MaterialInstanceSlotAssignmentRecord {
    type Reader<'a> = MaterialInstanceSlotAssignmentRef<'a>;
    type Writer<'a> = MaterialInstanceSlotAssignmentRefMut<'a>;
    type Accessor = MaterialInstanceSlotAssignmentAccessor;

    fn schema_name() -> &'static str {
        "MaterialInstanceSlotAssignment"
    }
}

impl MaterialInstanceSlotAssignmentRecord {
    pub fn array_index(self: &Self) -> U32Field {
        U32Field::new(self.0.push("array_index"), &self.1)
    }

    pub fn buffer_data(self: &Self) -> NullableField<BytesField> {
        NullableField::<BytesField>::new(self.0.push("buffer_data"), &self.1)
    }

    pub fn image(self: &Self) -> AssetRefField {
        AssetRefField::new(self.0.push("image"), &self.1)
    }

    pub fn sampler(self: &Self) -> StringField {
        StringField::new(self.0.push("sampler"), &self.1)
    }

    pub fn slot_name(self: &Self) -> StringField {
        StringField::new(self.0.push("slot_name"), &self.1)
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
pub struct MaterialPassRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for MaterialPassRef<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: DataContainerRef<'a>,
    ) -> Self {
        MaterialPassRef(property_path, data_container)
    }
}

impl<'a> RecordRef for MaterialPassRef<'a> {
    fn schema_name() -> &'static str {
        "MaterialPass"
    }
}

impl<'a> MaterialPassRef<'a> {
    pub fn fixed_function_state(&self) -> StringFieldRef {
        StringFieldRef::new(self.0.push("fixed_function_state"), self.1.clone())
    }

    pub fn fragment_stage(&self) -> GraphicsPipelineShaderStageRef {
        GraphicsPipelineShaderStageRef::new(self.0.push("fragment_stage"), self.1.clone())
    }

    pub fn name(&self) -> StringFieldRef {
        StringFieldRef::new(self.0.push("name"), self.1.clone())
    }

    pub fn phase(&self) -> StringFieldRef {
        StringFieldRef::new(self.0.push("phase"), self.1.clone())
    }

    pub fn vertex_stage(&self) -> GraphicsPipelineShaderStageRef {
        GraphicsPipelineShaderStageRef::new(self.0.push("vertex_stage"), self.1.clone())
    }
}
pub struct MaterialPassRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for MaterialPassRefMut<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<DataContainerRefMut<'a>>>,
    ) -> Self {
        MaterialPassRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for MaterialPassRefMut<'a> {
    fn schema_name() -> &'static str {
        "MaterialPass"
    }
}

impl<'a> MaterialPassRefMut<'a> {
    pub fn fixed_function_state(self: &'a Self) -> StringFieldRefMut {
        StringFieldRefMut::new(self.0.push("fixed_function_state"), &self.1)
    }

    pub fn fragment_stage(self: &'a Self) -> GraphicsPipelineShaderStageRefMut {
        GraphicsPipelineShaderStageRefMut::new(self.0.push("fragment_stage"), &self.1)
    }

    pub fn name(self: &'a Self) -> StringFieldRefMut {
        StringFieldRefMut::new(self.0.push("name"), &self.1)
    }

    pub fn phase(self: &'a Self) -> StringFieldRefMut {
        StringFieldRefMut::new(self.0.push("phase"), &self.1)
    }

    pub fn vertex_stage(self: &'a Self) -> GraphicsPipelineShaderStageRefMut {
        GraphicsPipelineShaderStageRefMut::new(self.0.push("vertex_stage"), &self.1)
    }
}
pub struct MaterialPassRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for MaterialPassRecord {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<Option<DataContainer>>>,
    ) -> Self {
        MaterialPassRecord(property_path, data_container.clone())
    }
}

impl Record for MaterialPassRecord {
    type Reader<'a> = MaterialPassRef<'a>;
    type Writer<'a> = MaterialPassRefMut<'a>;
    type Accessor = MaterialPassAccessor;

    fn schema_name() -> &'static str {
        "MaterialPass"
    }
}

impl MaterialPassRecord {
    pub fn fixed_function_state(self: &Self) -> StringField {
        StringField::new(self.0.push("fixed_function_state"), &self.1)
    }

    pub fn fragment_stage(self: &Self) -> GraphicsPipelineShaderStageRecord {
        GraphicsPipelineShaderStageRecord::new(self.0.push("fragment_stage"), &self.1)
    }

    pub fn name(self: &Self) -> StringField {
        StringField::new(self.0.push("name"), &self.1)
    }

    pub fn phase(self: &Self) -> StringField {
        StringField::new(self.0.push("phase"), &self.1)
    }

    pub fn vertex_stage(self: &Self) -> GraphicsPipelineShaderStageRecord {
        GraphicsPipelineShaderStageRecord::new(self.0.push("vertex_stage"), &self.1)
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

impl ShaderPackageAssetAccessor {}
pub struct ShaderPackageAssetRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for ShaderPackageAssetRef<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: DataContainerRef<'a>,
    ) -> Self {
        ShaderPackageAssetRef(property_path, data_container)
    }
}

impl<'a> RecordRef for ShaderPackageAssetRef<'a> {
    fn schema_name() -> &'static str {
        "ShaderPackageAsset"
    }
}

impl<'a> ShaderPackageAssetRef<'a> {}
pub struct ShaderPackageAssetRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for ShaderPackageAssetRefMut<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<DataContainerRefMut<'a>>>,
    ) -> Self {
        ShaderPackageAssetRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for ShaderPackageAssetRefMut<'a> {
    fn schema_name() -> &'static str {
        "ShaderPackageAsset"
    }
}

impl<'a> ShaderPackageAssetRefMut<'a> {}
pub struct ShaderPackageAssetRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for ShaderPackageAssetRecord {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<Option<DataContainer>>>,
    ) -> Self {
        ShaderPackageAssetRecord(property_path, data_container.clone())
    }
}

impl Record for ShaderPackageAssetRecord {
    type Reader<'a> = ShaderPackageAssetRef<'a>;
    type Writer<'a> = ShaderPackageAssetRefMut<'a>;
    type Accessor = ShaderPackageAssetAccessor;

    fn schema_name() -> &'static str {
        "ShaderPackageAsset"
    }
}

impl ShaderPackageAssetRecord {}
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
pub struct ShaderPackageImportedDataRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for ShaderPackageImportedDataRef<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: DataContainerRef<'a>,
    ) -> Self {
        ShaderPackageImportedDataRef(property_path, data_container)
    }
}

impl<'a> RecordRef for ShaderPackageImportedDataRef<'a> {
    fn schema_name() -> &'static str {
        "ShaderPackageImportedData"
    }
}

impl<'a> ShaderPackageImportedDataRef<'a> {
    pub fn bytes(&self) -> BytesFieldRef {
        BytesFieldRef::new(self.0.push("bytes"), self.1.clone())
    }
}
pub struct ShaderPackageImportedDataRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for ShaderPackageImportedDataRefMut<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<DataContainerRefMut<'a>>>,
    ) -> Self {
        ShaderPackageImportedDataRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for ShaderPackageImportedDataRefMut<'a> {
    fn schema_name() -> &'static str {
        "ShaderPackageImportedData"
    }
}

impl<'a> ShaderPackageImportedDataRefMut<'a> {
    pub fn bytes(self: &'a Self) -> BytesFieldRefMut {
        BytesFieldRefMut::new(self.0.push("bytes"), &self.1)
    }
}
pub struct ShaderPackageImportedDataRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for ShaderPackageImportedDataRecord {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<Option<DataContainer>>>,
    ) -> Self {
        ShaderPackageImportedDataRecord(property_path, data_container.clone())
    }
}

impl Record for ShaderPackageImportedDataRecord {
    type Reader<'a> = ShaderPackageImportedDataRef<'a>;
    type Writer<'a> = ShaderPackageImportedDataRefMut<'a>;
    type Accessor = ShaderPackageImportedDataAccessor;

    fn schema_name() -> &'static str {
        "ShaderPackageImportedData"
    }
}

impl ShaderPackageImportedDataRecord {
    pub fn bytes(self: &Self) -> BytesField {
        BytesField::new(self.0.push("bytes"), &self.1)
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
pub struct Vec3Ref<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for Vec3Ref<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: DataContainerRef<'a>,
    ) -> Self {
        Vec3Ref(property_path, data_container)
    }
}

impl<'a> RecordRef for Vec3Ref<'a> {
    fn schema_name() -> &'static str {
        "Vec3"
    }
}

impl<'a> Vec3Ref<'a> {
    pub fn x(&self) -> F32FieldRef {
        F32FieldRef::new(self.0.push("x"), self.1.clone())
    }

    pub fn y(&self) -> F32FieldRef {
        F32FieldRef::new(self.0.push("y"), self.1.clone())
    }

    pub fn z(&self) -> F32FieldRef {
        F32FieldRef::new(self.0.push("z"), self.1.clone())
    }
}
pub struct Vec3RefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for Vec3RefMut<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<DataContainerRefMut<'a>>>,
    ) -> Self {
        Vec3RefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for Vec3RefMut<'a> {
    fn schema_name() -> &'static str {
        "Vec3"
    }
}

impl<'a> Vec3RefMut<'a> {
    pub fn x(self: &'a Self) -> F32FieldRefMut {
        F32FieldRefMut::new(self.0.push("x"), &self.1)
    }

    pub fn y(self: &'a Self) -> F32FieldRefMut {
        F32FieldRefMut::new(self.0.push("y"), &self.1)
    }

    pub fn z(self: &'a Self) -> F32FieldRefMut {
        F32FieldRefMut::new(self.0.push("z"), &self.1)
    }
}
pub struct Vec3Record(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for Vec3Record {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<Option<DataContainer>>>,
    ) -> Self {
        Vec3Record(property_path, data_container.clone())
    }
}

impl Record for Vec3Record {
    type Reader<'a> = Vec3Ref<'a>;
    type Writer<'a> = Vec3RefMut<'a>;
    type Accessor = Vec3Accessor;

    fn schema_name() -> &'static str {
        "Vec3"
    }
}

impl Vec3Record {
    pub fn x(self: &Self) -> F32Field {
        F32Field::new(self.0.push("x"), &self.1)
    }

    pub fn y(self: &Self) -> F32Field {
        F32Field::new(self.0.push("y"), &self.1)
    }

    pub fn z(self: &Self) -> F32Field {
        F32Field::new(self.0.push("z"), &self.1)
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
pub struct Vec4Ref<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for Vec4Ref<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: DataContainerRef<'a>,
    ) -> Self {
        Vec4Ref(property_path, data_container)
    }
}

impl<'a> RecordRef for Vec4Ref<'a> {
    fn schema_name() -> &'static str {
        "Vec4"
    }
}

impl<'a> Vec4Ref<'a> {
    pub fn w(&self) -> F32FieldRef {
        F32FieldRef::new(self.0.push("w"), self.1.clone())
    }

    pub fn x(&self) -> F32FieldRef {
        F32FieldRef::new(self.0.push("x"), self.1.clone())
    }

    pub fn y(&self) -> F32FieldRef {
        F32FieldRef::new(self.0.push("y"), self.1.clone())
    }

    pub fn z(&self) -> F32FieldRef {
        F32FieldRef::new(self.0.push("z"), self.1.clone())
    }
}
pub struct Vec4RefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for Vec4RefMut<'a> {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<DataContainerRefMut<'a>>>,
    ) -> Self {
        Vec4RefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for Vec4RefMut<'a> {
    fn schema_name() -> &'static str {
        "Vec4"
    }
}

impl<'a> Vec4RefMut<'a> {
    pub fn w(self: &'a Self) -> F32FieldRefMut {
        F32FieldRefMut::new(self.0.push("w"), &self.1)
    }

    pub fn x(self: &'a Self) -> F32FieldRefMut {
        F32FieldRefMut::new(self.0.push("x"), &self.1)
    }

    pub fn y(self: &'a Self) -> F32FieldRefMut {
        F32FieldRefMut::new(self.0.push("y"), &self.1)
    }

    pub fn z(self: &'a Self) -> F32FieldRefMut {
        F32FieldRefMut::new(self.0.push("z"), &self.1)
    }
}
pub struct Vec4Record(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for Vec4Record {
    fn new(
        property_path: PropertyPath,
        data_container: &Rc<RefCell<Option<DataContainer>>>,
    ) -> Self {
        Vec4Record(property_path, data_container.clone())
    }
}

impl Record for Vec4Record {
    type Reader<'a> = Vec4Ref<'a>;
    type Writer<'a> = Vec4RefMut<'a>;
    type Accessor = Vec4Accessor;

    fn schema_name() -> &'static str {
        "Vec4"
    }
}

impl Vec4Record {
    pub fn w(self: &Self) -> F32Field {
        F32Field::new(self.0.push("w"), &self.1)
    }

    pub fn x(self: &Self) -> F32Field {
        F32Field::new(self.0.push("x"), &self.1)
    }

    pub fn y(self: &Self) -> F32Field {
        F32Field::new(self.0.push("y"), &self.1)
    }

    pub fn z(self: &Self) -> F32Field {
        F32Field::new(self.0.push("z"), &self.1)
    }
}
