// This file generated automatically by hydrate-codegen. Do not make manual edits. Use include!() to place these types in the intended location.
#[derive(Default)]
pub struct BlenderAnimAssetAccessor(PropertyPath);

impl FieldAccessor for BlenderAnimAssetAccessor {
    fn new(property_path: PropertyPath) -> Self {
        BlenderAnimAssetAccessor(property_path)
    }
}

impl RecordAccessor for BlenderAnimAssetAccessor {
    fn schema_name() -> &'static str {
        "BlenderAnimAsset"
    }
}

impl BlenderAnimAssetAccessor {
}
pub struct BlenderAnimAssetReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for BlenderAnimAssetReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        BlenderAnimAssetReader(property_path, data_container)
    }
}

impl<'a> RecordReader for BlenderAnimAssetReader<'a> {
    fn schema_name() -> &'static str {
        "BlenderAnimAsset"
    }
}

impl<'a> BlenderAnimAssetReader<'a> {
}
pub struct BlenderAnimAssetWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for BlenderAnimAssetWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        BlenderAnimAssetWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for BlenderAnimAssetWriter<'a> {
    fn schema_name() -> &'static str {
        "BlenderAnimAsset"
    }
}

impl<'a> BlenderAnimAssetWriter<'a> {
}
pub struct BlenderAnimAssetOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for BlenderAnimAssetOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        BlenderAnimAssetOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for BlenderAnimAssetOwned {
    fn schema_name() -> &'static str {
        "BlenderAnimAsset"
    }
}

impl BlenderAnimAssetOwned {
}
#[derive(Default)]
pub struct BlenderAnimImportedDataAccessor(PropertyPath);

impl FieldAccessor for BlenderAnimImportedDataAccessor {
    fn new(property_path: PropertyPath) -> Self {
        BlenderAnimImportedDataAccessor(property_path)
    }
}

impl RecordAccessor for BlenderAnimImportedDataAccessor {
    fn schema_name() -> &'static str {
        "BlenderAnimImportedData"
    }
}

impl BlenderAnimImportedDataAccessor {
    pub fn json_string(&self) -> StringFieldAccessor {
        StringFieldAccessor::new(self.0.push("json_string"))
    }
}
pub struct BlenderAnimImportedDataReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for BlenderAnimImportedDataReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        BlenderAnimImportedDataReader(property_path, data_container)
    }
}

impl<'a> RecordReader for BlenderAnimImportedDataReader<'a> {
    fn schema_name() -> &'static str {
        "BlenderAnimImportedData"
    }
}

impl<'a> BlenderAnimImportedDataReader<'a> {
    pub fn json_string(&self) -> StringFieldReader {
        StringFieldReader::new(self.0.push("json_string"), self.1)
    }
}
pub struct BlenderAnimImportedDataWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for BlenderAnimImportedDataWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        BlenderAnimImportedDataWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for BlenderAnimImportedDataWriter<'a> {
    fn schema_name() -> &'static str {
        "BlenderAnimImportedData"
    }
}

impl<'a> BlenderAnimImportedDataWriter<'a> {
    pub fn json_string(self: &'a Self) -> StringFieldWriter {
        StringFieldWriter::new(self.0.push("json_string"), &self.1)
    }
}
pub struct BlenderAnimImportedDataOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for BlenderAnimImportedDataOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        BlenderAnimImportedDataOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for BlenderAnimImportedDataOwned {
    fn schema_name() -> &'static str {
        "BlenderAnimImportedData"
    }
}

impl BlenderAnimImportedDataOwned {
    pub fn json_string(self: &Self) -> StringFieldOwned {
        StringFieldOwned::new(self.0.push("json_string"), &self.1)
    }
}
#[derive(Default)]
pub struct FontAssetAccessor(PropertyPath);

impl FieldAccessor for FontAssetAccessor {
    fn new(property_path: PropertyPath) -> Self {
        FontAssetAccessor(property_path)
    }
}

impl RecordAccessor for FontAssetAccessor {
    fn schema_name() -> &'static str {
        "FontAsset"
    }
}

impl FontAssetAccessor {
}
pub struct FontAssetReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for FontAssetReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        FontAssetReader(property_path, data_container)
    }
}

impl<'a> RecordReader for FontAssetReader<'a> {
    fn schema_name() -> &'static str {
        "FontAsset"
    }
}

impl<'a> FontAssetReader<'a> {
}
pub struct FontAssetWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for FontAssetWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        FontAssetWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for FontAssetWriter<'a> {
    fn schema_name() -> &'static str {
        "FontAsset"
    }
}

impl<'a> FontAssetWriter<'a> {
}
pub struct FontAssetOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for FontAssetOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        FontAssetOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for FontAssetOwned {
    fn schema_name() -> &'static str {
        "FontAsset"
    }
}

impl FontAssetOwned {
}
#[derive(Default)]
pub struct FontImportedDataAccessor(PropertyPath);

impl FieldAccessor for FontImportedDataAccessor {
    fn new(property_path: PropertyPath) -> Self {
        FontImportedDataAccessor(property_path)
    }
}

impl RecordAccessor for FontImportedDataAccessor {
    fn schema_name() -> &'static str {
        "FontImportedData"
    }
}

impl FontImportedDataAccessor {
    pub fn bytes(&self) -> BytesFieldAccessor {
        BytesFieldAccessor::new(self.0.push("bytes"))
    }
}
pub struct FontImportedDataReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for FontImportedDataReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        FontImportedDataReader(property_path, data_container)
    }
}

impl<'a> RecordReader for FontImportedDataReader<'a> {
    fn schema_name() -> &'static str {
        "FontImportedData"
    }
}

impl<'a> FontImportedDataReader<'a> {
    pub fn bytes(&self) -> BytesFieldReader {
        BytesFieldReader::new(self.0.push("bytes"), self.1)
    }
}
pub struct FontImportedDataWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for FontImportedDataWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        FontImportedDataWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for FontImportedDataWriter<'a> {
    fn schema_name() -> &'static str {
        "FontImportedData"
    }
}

impl<'a> FontImportedDataWriter<'a> {
    pub fn bytes(self: &'a Self) -> BytesFieldWriter {
        BytesFieldWriter::new(self.0.push("bytes"), &self.1)
    }
}
pub struct FontImportedDataOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for FontImportedDataOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        FontImportedDataOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for FontImportedDataOwned {
    fn schema_name() -> &'static str {
        "FontImportedData"
    }
}

impl FontImportedDataOwned {
    pub fn bytes(self: &Self) -> BytesFieldOwned {
        BytesFieldOwned::new(self.0.push("bytes"), &self.1)
    }
}
#[derive(Default)]
pub struct LdtkAssetAccessor(PropertyPath);

impl FieldAccessor for LdtkAssetAccessor {
    fn new(property_path: PropertyPath) -> Self {
        LdtkAssetAccessor(property_path)
    }
}

impl RecordAccessor for LdtkAssetAccessor {
    fn schema_name() -> &'static str {
        "LdtkAsset"
    }
}

impl LdtkAssetAccessor {
}
pub struct LdtkAssetReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for LdtkAssetReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        LdtkAssetReader(property_path, data_container)
    }
}

impl<'a> RecordReader for LdtkAssetReader<'a> {
    fn schema_name() -> &'static str {
        "LdtkAsset"
    }
}

impl<'a> LdtkAssetReader<'a> {
}
pub struct LdtkAssetWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for LdtkAssetWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        LdtkAssetWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for LdtkAssetWriter<'a> {
    fn schema_name() -> &'static str {
        "LdtkAsset"
    }
}

impl<'a> LdtkAssetWriter<'a> {
}
pub struct LdtkAssetOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for LdtkAssetOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        LdtkAssetOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for LdtkAssetOwned {
    fn schema_name() -> &'static str {
        "LdtkAsset"
    }
}

impl LdtkAssetOwned {
}
#[derive(Default)]
pub struct LdtkImportDataAccessor(PropertyPath);

impl FieldAccessor for LdtkImportDataAccessor {
    fn new(property_path: PropertyPath) -> Self {
        LdtkImportDataAccessor(property_path)
    }
}

impl RecordAccessor for LdtkImportDataAccessor {
    fn schema_name() -> &'static str {
        "LdtkImportData"
    }
}

impl LdtkImportDataAccessor {
    pub fn json_data(&self) -> StringFieldAccessor {
        StringFieldAccessor::new(self.0.push("json_data"))
    }
}
pub struct LdtkImportDataReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for LdtkImportDataReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        LdtkImportDataReader(property_path, data_container)
    }
}

impl<'a> RecordReader for LdtkImportDataReader<'a> {
    fn schema_name() -> &'static str {
        "LdtkImportData"
    }
}

impl<'a> LdtkImportDataReader<'a> {
    pub fn json_data(&self) -> StringFieldReader {
        StringFieldReader::new(self.0.push("json_data"), self.1)
    }
}
pub struct LdtkImportDataWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for LdtkImportDataWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        LdtkImportDataWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for LdtkImportDataWriter<'a> {
    fn schema_name() -> &'static str {
        "LdtkImportData"
    }
}

impl<'a> LdtkImportDataWriter<'a> {
    pub fn json_data(self: &'a Self) -> StringFieldWriter {
        StringFieldWriter::new(self.0.push("json_data"), &self.1)
    }
}
pub struct LdtkImportDataOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for LdtkImportDataOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        LdtkImportDataOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for LdtkImportDataOwned {
    fn schema_name() -> &'static str {
        "LdtkImportData"
    }
}

impl LdtkImportDataOwned {
    pub fn json_data(self: &Self) -> StringFieldOwned {
        StringFieldOwned::new(self.0.push("json_data"), &self.1)
    }
}
#[derive(Copy, Clone)]
pub enum MeshAdvBlendMethodEnum {
    Opaque,
    AlphaClip,
    AlphaBlend,
}

impl Enum for MeshAdvBlendMethodEnum {
    fn to_symbol_name(&self) -> &'static str {
        match self {
            MeshAdvBlendMethodEnum::Opaque => "Opaque",
            MeshAdvBlendMethodEnum::AlphaClip => "AlphaClip",
            MeshAdvBlendMethodEnum::AlphaBlend => "AlphaBlend",
        }
    }

    fn from_symbol_name(str: &str) -> Option<MeshAdvBlendMethodEnum> {
        match str {
            "Opaque" => Some(MeshAdvBlendMethodEnum::Opaque),
            "OPAQUE" => Some(MeshAdvBlendMethodEnum::Opaque),
            "AlphaClip" => Some(MeshAdvBlendMethodEnum::AlphaClip),
            "ALPHA_CLIP" => Some(MeshAdvBlendMethodEnum::AlphaClip),
            "AlphaBlend" => Some(MeshAdvBlendMethodEnum::AlphaBlend),
            "ALPHA_BLEND" => Some(MeshAdvBlendMethodEnum::AlphaBlend),
            "BLEND" => Some(MeshAdvBlendMethodEnum::AlphaBlend),
            _ => None,
        }
    }
}

impl MeshAdvBlendMethodEnum {
    pub fn schema_name() -> &'static str {
        "MeshAdvBlendMethod"
    }
}
#[derive(Copy, Clone)]
pub enum MeshAdvIndexTypeEnum {
    Uint16,
    Uint32,
}

impl Enum for MeshAdvIndexTypeEnum {
    fn to_symbol_name(&self) -> &'static str {
        match self {
            MeshAdvIndexTypeEnum::Uint16 => "Uint16",
            MeshAdvIndexTypeEnum::Uint32 => "Uint32",
        }
    }

    fn from_symbol_name(str: &str) -> Option<MeshAdvIndexTypeEnum> {
        match str {
            "Uint16" => Some(MeshAdvIndexTypeEnum::Uint16),
            "Uint32" => Some(MeshAdvIndexTypeEnum::Uint32),
            _ => None,
        }
    }
}

impl MeshAdvIndexTypeEnum {
    pub fn schema_name() -> &'static str {
        "MeshAdvIndexType"
    }
}
#[derive(Default)]
pub struct MeshAdvMaterialAssetAccessor(PropertyPath);

impl FieldAccessor for MeshAdvMaterialAssetAccessor {
    fn new(property_path: PropertyPath) -> Self {
        MeshAdvMaterialAssetAccessor(property_path)
    }
}

impl RecordAccessor for MeshAdvMaterialAssetAccessor {
    fn schema_name() -> &'static str {
        "MeshAdvMaterialAsset"
    }
}

impl MeshAdvMaterialAssetAccessor {
    pub fn alpha_threshold(&self) -> F32FieldAccessor {
        F32FieldAccessor::new(self.0.push("alpha_threshold"))
    }

    pub fn backface_culling(&self) -> BooleanFieldAccessor {
        BooleanFieldAccessor::new(self.0.push("backface_culling"))
    }

    pub fn base_color_factor(&self) -> Vec4Accessor {
        Vec4Accessor::new(self.0.push("base_color_factor"))
    }

    pub fn blend_method(&self) -> EnumFieldAccessor::<MeshAdvBlendMethodEnum> {
        EnumFieldAccessor::<MeshAdvBlendMethodEnum>::new(self.0.push("blend_method"))
    }

    pub fn color_texture(&self) -> AssetRefFieldAccessor {
        AssetRefFieldAccessor::new(self.0.push("color_texture"))
    }

    pub fn color_texture_has_alpha_channel(&self) -> BooleanFieldAccessor {
        BooleanFieldAccessor::new(self.0.push("color_texture_has_alpha_channel"))
    }

    pub fn emissive_factor(&self) -> Vec3Accessor {
        Vec3Accessor::new(self.0.push("emissive_factor"))
    }

    pub fn emissive_texture(&self) -> AssetRefFieldAccessor {
        AssetRefFieldAccessor::new(self.0.push("emissive_texture"))
    }

    pub fn metallic_factor(&self) -> F32FieldAccessor {
        F32FieldAccessor::new(self.0.push("metallic_factor"))
    }

    pub fn metallic_roughness_texture(&self) -> AssetRefFieldAccessor {
        AssetRefFieldAccessor::new(self.0.push("metallic_roughness_texture"))
    }

    pub fn normal_texture(&self) -> AssetRefFieldAccessor {
        AssetRefFieldAccessor::new(self.0.push("normal_texture"))
    }

    pub fn normal_texture_scale(&self) -> F32FieldAccessor {
        F32FieldAccessor::new(self.0.push("normal_texture_scale"))
    }

    pub fn occlusion_texture(&self) -> AssetRefFieldAccessor {
        AssetRefFieldAccessor::new(self.0.push("occlusion_texture"))
    }

    pub fn roughness_factor(&self) -> F32FieldAccessor {
        F32FieldAccessor::new(self.0.push("roughness_factor"))
    }

    pub fn shadow_method(&self) -> EnumFieldAccessor::<MeshAdvShadowMethodEnum> {
        EnumFieldAccessor::<MeshAdvShadowMethodEnum>::new(self.0.push("shadow_method"))
    }
}
pub struct MeshAdvMaterialAssetReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for MeshAdvMaterialAssetReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MeshAdvMaterialAssetReader(property_path, data_container)
    }
}

impl<'a> RecordReader for MeshAdvMaterialAssetReader<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvMaterialAsset"
    }
}

impl<'a> MeshAdvMaterialAssetReader<'a> {
    pub fn alpha_threshold(&self) -> F32FieldReader {
        F32FieldReader::new(self.0.push("alpha_threshold"), self.1)
    }

    pub fn backface_culling(&self) -> BooleanFieldReader {
        BooleanFieldReader::new(self.0.push("backface_culling"), self.1)
    }

    pub fn base_color_factor(&self) -> Vec4Reader {
        Vec4Reader::new(self.0.push("base_color_factor"), self.1)
    }

    pub fn blend_method(&self) -> EnumFieldReader::<MeshAdvBlendMethodEnum> {
        EnumFieldReader::<MeshAdvBlendMethodEnum>::new(self.0.push("blend_method"), self.1)
    }

    pub fn color_texture(&self) -> AssetRefFieldReader {
        AssetRefFieldReader::new(self.0.push("color_texture"), self.1)
    }

    pub fn color_texture_has_alpha_channel(&self) -> BooleanFieldReader {
        BooleanFieldReader::new(self.0.push("color_texture_has_alpha_channel"), self.1)
    }

    pub fn emissive_factor(&self) -> Vec3Reader {
        Vec3Reader::new(self.0.push("emissive_factor"), self.1)
    }

    pub fn emissive_texture(&self) -> AssetRefFieldReader {
        AssetRefFieldReader::new(self.0.push("emissive_texture"), self.1)
    }

    pub fn metallic_factor(&self) -> F32FieldReader {
        F32FieldReader::new(self.0.push("metallic_factor"), self.1)
    }

    pub fn metallic_roughness_texture(&self) -> AssetRefFieldReader {
        AssetRefFieldReader::new(self.0.push("metallic_roughness_texture"), self.1)
    }

    pub fn normal_texture(&self) -> AssetRefFieldReader {
        AssetRefFieldReader::new(self.0.push("normal_texture"), self.1)
    }

    pub fn normal_texture_scale(&self) -> F32FieldReader {
        F32FieldReader::new(self.0.push("normal_texture_scale"), self.1)
    }

    pub fn occlusion_texture(&self) -> AssetRefFieldReader {
        AssetRefFieldReader::new(self.0.push("occlusion_texture"), self.1)
    }

    pub fn roughness_factor(&self) -> F32FieldReader {
        F32FieldReader::new(self.0.push("roughness_factor"), self.1)
    }

    pub fn shadow_method(&self) -> EnumFieldReader::<MeshAdvShadowMethodEnum> {
        EnumFieldReader::<MeshAdvShadowMethodEnum>::new(self.0.push("shadow_method"), self.1)
    }
}
pub struct MeshAdvMaterialAssetWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for MeshAdvMaterialAssetWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MeshAdvMaterialAssetWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for MeshAdvMaterialAssetWriter<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvMaterialAsset"
    }
}

impl<'a> MeshAdvMaterialAssetWriter<'a> {
    pub fn alpha_threshold(self: &'a Self) -> F32FieldWriter {
        F32FieldWriter::new(self.0.push("alpha_threshold"), &self.1)
    }

    pub fn backface_culling(self: &'a Self) -> BooleanFieldWriter {
        BooleanFieldWriter::new(self.0.push("backface_culling"), &self.1)
    }

    pub fn base_color_factor(self: &'a Self) -> Vec4Writer {
        Vec4Writer::new(self.0.push("base_color_factor"), &self.1)
    }

    pub fn blend_method(self: &'a Self) -> EnumFieldWriter::<MeshAdvBlendMethodEnum> {
        EnumFieldWriter::<MeshAdvBlendMethodEnum>::new(self.0.push("blend_method"), &self.1)
    }

    pub fn color_texture(self: &'a Self) -> AssetRefFieldWriter {
        AssetRefFieldWriter::new(self.0.push("color_texture"), &self.1)
    }

    pub fn color_texture_has_alpha_channel(self: &'a Self) -> BooleanFieldWriter {
        BooleanFieldWriter::new(self.0.push("color_texture_has_alpha_channel"), &self.1)
    }

    pub fn emissive_factor(self: &'a Self) -> Vec3Writer {
        Vec3Writer::new(self.0.push("emissive_factor"), &self.1)
    }

    pub fn emissive_texture(self: &'a Self) -> AssetRefFieldWriter {
        AssetRefFieldWriter::new(self.0.push("emissive_texture"), &self.1)
    }

    pub fn metallic_factor(self: &'a Self) -> F32FieldWriter {
        F32FieldWriter::new(self.0.push("metallic_factor"), &self.1)
    }

    pub fn metallic_roughness_texture(self: &'a Self) -> AssetRefFieldWriter {
        AssetRefFieldWriter::new(self.0.push("metallic_roughness_texture"), &self.1)
    }

    pub fn normal_texture(self: &'a Self) -> AssetRefFieldWriter {
        AssetRefFieldWriter::new(self.0.push("normal_texture"), &self.1)
    }

    pub fn normal_texture_scale(self: &'a Self) -> F32FieldWriter {
        F32FieldWriter::new(self.0.push("normal_texture_scale"), &self.1)
    }

    pub fn occlusion_texture(self: &'a Self) -> AssetRefFieldWriter {
        AssetRefFieldWriter::new(self.0.push("occlusion_texture"), &self.1)
    }

    pub fn roughness_factor(self: &'a Self) -> F32FieldWriter {
        F32FieldWriter::new(self.0.push("roughness_factor"), &self.1)
    }

    pub fn shadow_method(self: &'a Self) -> EnumFieldWriter::<MeshAdvShadowMethodEnum> {
        EnumFieldWriter::<MeshAdvShadowMethodEnum>::new(self.0.push("shadow_method"), &self.1)
    }
}
pub struct MeshAdvMaterialAssetOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for MeshAdvMaterialAssetOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        MeshAdvMaterialAssetOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for MeshAdvMaterialAssetOwned {
    fn schema_name() -> &'static str {
        "MeshAdvMaterialAsset"
    }
}

impl MeshAdvMaterialAssetOwned {
    pub fn alpha_threshold(self: &Self) -> F32FieldOwned {
        F32FieldOwned::new(self.0.push("alpha_threshold"), &self.1)
    }

    pub fn backface_culling(self: &Self) -> BooleanFieldOwned {
        BooleanFieldOwned::new(self.0.push("backface_culling"), &self.1)
    }

    pub fn base_color_factor(self: &Self) -> Vec4Owned {
        Vec4Owned::new(self.0.push("base_color_factor"), &self.1)
    }

    pub fn blend_method(self: &Self) -> EnumFieldOwned::<MeshAdvBlendMethodEnum> {
        EnumFieldOwned::<MeshAdvBlendMethodEnum>::new(self.0.push("blend_method"), &self.1)
    }

    pub fn color_texture(self: &Self) -> AssetRefFieldOwned {
        AssetRefFieldOwned::new(self.0.push("color_texture"), &self.1)
    }

    pub fn color_texture_has_alpha_channel(self: &Self) -> BooleanFieldOwned {
        BooleanFieldOwned::new(self.0.push("color_texture_has_alpha_channel"), &self.1)
    }

    pub fn emissive_factor(self: &Self) -> Vec3Owned {
        Vec3Owned::new(self.0.push("emissive_factor"), &self.1)
    }

    pub fn emissive_texture(self: &Self) -> AssetRefFieldOwned {
        AssetRefFieldOwned::new(self.0.push("emissive_texture"), &self.1)
    }

    pub fn metallic_factor(self: &Self) -> F32FieldOwned {
        F32FieldOwned::new(self.0.push("metallic_factor"), &self.1)
    }

    pub fn metallic_roughness_texture(self: &Self) -> AssetRefFieldOwned {
        AssetRefFieldOwned::new(self.0.push("metallic_roughness_texture"), &self.1)
    }

    pub fn normal_texture(self: &Self) -> AssetRefFieldOwned {
        AssetRefFieldOwned::new(self.0.push("normal_texture"), &self.1)
    }

    pub fn normal_texture_scale(self: &Self) -> F32FieldOwned {
        F32FieldOwned::new(self.0.push("normal_texture_scale"), &self.1)
    }

    pub fn occlusion_texture(self: &Self) -> AssetRefFieldOwned {
        AssetRefFieldOwned::new(self.0.push("occlusion_texture"), &self.1)
    }

    pub fn roughness_factor(self: &Self) -> F32FieldOwned {
        F32FieldOwned::new(self.0.push("roughness_factor"), &self.1)
    }

    pub fn shadow_method(self: &Self) -> EnumFieldOwned::<MeshAdvShadowMethodEnum> {
        EnumFieldOwned::<MeshAdvShadowMethodEnum>::new(self.0.push("shadow_method"), &self.1)
    }
}
#[derive(Default)]
pub struct MeshAdvMaterialImportedDataAccessor(PropertyPath);

impl FieldAccessor for MeshAdvMaterialImportedDataAccessor {
    fn new(property_path: PropertyPath) -> Self {
        MeshAdvMaterialImportedDataAccessor(property_path)
    }
}

impl RecordAccessor for MeshAdvMaterialImportedDataAccessor {
    fn schema_name() -> &'static str {
        "MeshAdvMaterialImportedData"
    }
}

impl MeshAdvMaterialImportedDataAccessor {
}
pub struct MeshAdvMaterialImportedDataReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for MeshAdvMaterialImportedDataReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MeshAdvMaterialImportedDataReader(property_path, data_container)
    }
}

impl<'a> RecordReader for MeshAdvMaterialImportedDataReader<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvMaterialImportedData"
    }
}

impl<'a> MeshAdvMaterialImportedDataReader<'a> {
}
pub struct MeshAdvMaterialImportedDataWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for MeshAdvMaterialImportedDataWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MeshAdvMaterialImportedDataWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for MeshAdvMaterialImportedDataWriter<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvMaterialImportedData"
    }
}

impl<'a> MeshAdvMaterialImportedDataWriter<'a> {
}
pub struct MeshAdvMaterialImportedDataOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for MeshAdvMaterialImportedDataOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        MeshAdvMaterialImportedDataOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for MeshAdvMaterialImportedDataOwned {
    fn schema_name() -> &'static str {
        "MeshAdvMaterialImportedData"
    }
}

impl MeshAdvMaterialImportedDataOwned {
}
#[derive(Default)]
pub struct MeshAdvMeshAssetAccessor(PropertyPath);

impl FieldAccessor for MeshAdvMeshAssetAccessor {
    fn new(property_path: PropertyPath) -> Self {
        MeshAdvMeshAssetAccessor(property_path)
    }
}

impl RecordAccessor for MeshAdvMeshAssetAccessor {
    fn schema_name() -> &'static str {
        "MeshAdvMeshAsset"
    }
}

impl MeshAdvMeshAssetAccessor {
    pub fn material_slots(&self) -> DynamicArrayFieldAccessor::<AssetRefFieldAccessor> {
        DynamicArrayFieldAccessor::<AssetRefFieldAccessor>::new(self.0.push("material_slots"))
    }
}
pub struct MeshAdvMeshAssetReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for MeshAdvMeshAssetReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MeshAdvMeshAssetReader(property_path, data_container)
    }
}

impl<'a> RecordReader for MeshAdvMeshAssetReader<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvMeshAsset"
    }
}

impl<'a> MeshAdvMeshAssetReader<'a> {
    pub fn material_slots(&self) -> DynamicArrayFieldReader::<AssetRefFieldReader> {
        DynamicArrayFieldReader::<AssetRefFieldReader>::new(self.0.push("material_slots"), self.1)
    }
}
pub struct MeshAdvMeshAssetWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for MeshAdvMeshAssetWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MeshAdvMeshAssetWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for MeshAdvMeshAssetWriter<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvMeshAsset"
    }
}

impl<'a> MeshAdvMeshAssetWriter<'a> {
    pub fn material_slots(self: &'a Self) -> DynamicArrayFieldWriter::<AssetRefFieldWriter> {
        DynamicArrayFieldWriter::<AssetRefFieldWriter>::new(self.0.push("material_slots"), &self.1)
    }
}
pub struct MeshAdvMeshAssetOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for MeshAdvMeshAssetOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        MeshAdvMeshAssetOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for MeshAdvMeshAssetOwned {
    fn schema_name() -> &'static str {
        "MeshAdvMeshAsset"
    }
}

impl MeshAdvMeshAssetOwned {
    pub fn material_slots(self: &Self) -> DynamicArrayFieldOwned::<AssetRefFieldOwned> {
        DynamicArrayFieldOwned::<AssetRefFieldOwned>::new(self.0.push("material_slots"), &self.1)
    }
}
#[derive(Default)]
pub struct MeshAdvMeshImportedDataAccessor(PropertyPath);

impl FieldAccessor for MeshAdvMeshImportedDataAccessor {
    fn new(property_path: PropertyPath) -> Self {
        MeshAdvMeshImportedDataAccessor(property_path)
    }
}

impl RecordAccessor for MeshAdvMeshImportedDataAccessor {
    fn schema_name() -> &'static str {
        "MeshAdvMeshImportedData"
    }
}

impl MeshAdvMeshImportedDataAccessor {
    pub fn mesh_parts(&self) -> DynamicArrayFieldAccessor::<MeshAdvMeshImportedDataMeshPartAccessor> {
        DynamicArrayFieldAccessor::<MeshAdvMeshImportedDataMeshPartAccessor>::new(self.0.push("mesh_parts"))
    }
}
pub struct MeshAdvMeshImportedDataReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for MeshAdvMeshImportedDataReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MeshAdvMeshImportedDataReader(property_path, data_container)
    }
}

impl<'a> RecordReader for MeshAdvMeshImportedDataReader<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvMeshImportedData"
    }
}

impl<'a> MeshAdvMeshImportedDataReader<'a> {
    pub fn mesh_parts(&self) -> DynamicArrayFieldReader::<MeshAdvMeshImportedDataMeshPartReader> {
        DynamicArrayFieldReader::<MeshAdvMeshImportedDataMeshPartReader>::new(self.0.push("mesh_parts"), self.1)
    }
}
pub struct MeshAdvMeshImportedDataWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for MeshAdvMeshImportedDataWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MeshAdvMeshImportedDataWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for MeshAdvMeshImportedDataWriter<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvMeshImportedData"
    }
}

impl<'a> MeshAdvMeshImportedDataWriter<'a> {
    pub fn mesh_parts(self: &'a Self) -> DynamicArrayFieldWriter::<MeshAdvMeshImportedDataMeshPartWriter> {
        DynamicArrayFieldWriter::<MeshAdvMeshImportedDataMeshPartWriter>::new(self.0.push("mesh_parts"), &self.1)
    }
}
pub struct MeshAdvMeshImportedDataOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for MeshAdvMeshImportedDataOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        MeshAdvMeshImportedDataOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for MeshAdvMeshImportedDataOwned {
    fn schema_name() -> &'static str {
        "MeshAdvMeshImportedData"
    }
}

impl MeshAdvMeshImportedDataOwned {
    pub fn mesh_parts(self: &Self) -> DynamicArrayFieldOwned::<MeshAdvMeshImportedDataMeshPartOwned> {
        DynamicArrayFieldOwned::<MeshAdvMeshImportedDataMeshPartOwned>::new(self.0.push("mesh_parts"), &self.1)
    }
}
#[derive(Default)]
pub struct MeshAdvMeshImportedDataMeshPartAccessor(PropertyPath);

impl FieldAccessor for MeshAdvMeshImportedDataMeshPartAccessor {
    fn new(property_path: PropertyPath) -> Self {
        MeshAdvMeshImportedDataMeshPartAccessor(property_path)
    }
}

impl RecordAccessor for MeshAdvMeshImportedDataMeshPartAccessor {
    fn schema_name() -> &'static str {
        "MeshAdvMeshImportedDataMeshPart"
    }
}

impl MeshAdvMeshImportedDataMeshPartAccessor {
    pub fn indices(&self) -> BytesFieldAccessor {
        BytesFieldAccessor::new(self.0.push("indices"))
    }

    pub fn material_index(&self) -> U32FieldAccessor {
        U32FieldAccessor::new(self.0.push("material_index"))
    }

    pub fn normals(&self) -> BytesFieldAccessor {
        BytesFieldAccessor::new(self.0.push("normals"))
    }

    pub fn positions(&self) -> BytesFieldAccessor {
        BytesFieldAccessor::new(self.0.push("positions"))
    }

    pub fn texture_coordinates(&self) -> BytesFieldAccessor {
        BytesFieldAccessor::new(self.0.push("texture_coordinates"))
    }
}
pub struct MeshAdvMeshImportedDataMeshPartReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for MeshAdvMeshImportedDataMeshPartReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MeshAdvMeshImportedDataMeshPartReader(property_path, data_container)
    }
}

impl<'a> RecordReader for MeshAdvMeshImportedDataMeshPartReader<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvMeshImportedDataMeshPart"
    }
}

impl<'a> MeshAdvMeshImportedDataMeshPartReader<'a> {
    pub fn indices(&self) -> BytesFieldReader {
        BytesFieldReader::new(self.0.push("indices"), self.1)
    }

    pub fn material_index(&self) -> U32FieldReader {
        U32FieldReader::new(self.0.push("material_index"), self.1)
    }

    pub fn normals(&self) -> BytesFieldReader {
        BytesFieldReader::new(self.0.push("normals"), self.1)
    }

    pub fn positions(&self) -> BytesFieldReader {
        BytesFieldReader::new(self.0.push("positions"), self.1)
    }

    pub fn texture_coordinates(&self) -> BytesFieldReader {
        BytesFieldReader::new(self.0.push("texture_coordinates"), self.1)
    }
}
pub struct MeshAdvMeshImportedDataMeshPartWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for MeshAdvMeshImportedDataMeshPartWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MeshAdvMeshImportedDataMeshPartWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for MeshAdvMeshImportedDataMeshPartWriter<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvMeshImportedDataMeshPart"
    }
}

impl<'a> MeshAdvMeshImportedDataMeshPartWriter<'a> {
    pub fn indices(self: &'a Self) -> BytesFieldWriter {
        BytesFieldWriter::new(self.0.push("indices"), &self.1)
    }

    pub fn material_index(self: &'a Self) -> U32FieldWriter {
        U32FieldWriter::new(self.0.push("material_index"), &self.1)
    }

    pub fn normals(self: &'a Self) -> BytesFieldWriter {
        BytesFieldWriter::new(self.0.push("normals"), &self.1)
    }

    pub fn positions(self: &'a Self) -> BytesFieldWriter {
        BytesFieldWriter::new(self.0.push("positions"), &self.1)
    }

    pub fn texture_coordinates(self: &'a Self) -> BytesFieldWriter {
        BytesFieldWriter::new(self.0.push("texture_coordinates"), &self.1)
    }
}
pub struct MeshAdvMeshImportedDataMeshPartOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for MeshAdvMeshImportedDataMeshPartOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        MeshAdvMeshImportedDataMeshPartOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for MeshAdvMeshImportedDataMeshPartOwned {
    fn schema_name() -> &'static str {
        "MeshAdvMeshImportedDataMeshPart"
    }
}

impl MeshAdvMeshImportedDataMeshPartOwned {
    pub fn indices(self: &Self) -> BytesFieldOwned {
        BytesFieldOwned::new(self.0.push("indices"), &self.1)
    }

    pub fn material_index(self: &Self) -> U32FieldOwned {
        U32FieldOwned::new(self.0.push("material_index"), &self.1)
    }

    pub fn normals(self: &Self) -> BytesFieldOwned {
        BytesFieldOwned::new(self.0.push("normals"), &self.1)
    }

    pub fn positions(self: &Self) -> BytesFieldOwned {
        BytesFieldOwned::new(self.0.push("positions"), &self.1)
    }

    pub fn texture_coordinates(self: &Self) -> BytesFieldOwned {
        BytesFieldOwned::new(self.0.push("texture_coordinates"), &self.1)
    }
}
#[derive(Default)]
pub struct MeshAdvModelAssetAccessor(PropertyPath);

impl FieldAccessor for MeshAdvModelAssetAccessor {
    fn new(property_path: PropertyPath) -> Self {
        MeshAdvModelAssetAccessor(property_path)
    }
}

impl RecordAccessor for MeshAdvModelAssetAccessor {
    fn schema_name() -> &'static str {
        "MeshAdvModelAsset"
    }
}

impl MeshAdvModelAssetAccessor {
    pub fn lods(&self) -> DynamicArrayFieldAccessor::<MeshAdvModelLodAccessor> {
        DynamicArrayFieldAccessor::<MeshAdvModelLodAccessor>::new(self.0.push("lods"))
    }
}
pub struct MeshAdvModelAssetReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for MeshAdvModelAssetReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MeshAdvModelAssetReader(property_path, data_container)
    }
}

impl<'a> RecordReader for MeshAdvModelAssetReader<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvModelAsset"
    }
}

impl<'a> MeshAdvModelAssetReader<'a> {
    pub fn lods(&self) -> DynamicArrayFieldReader::<MeshAdvModelLodReader> {
        DynamicArrayFieldReader::<MeshAdvModelLodReader>::new(self.0.push("lods"), self.1)
    }
}
pub struct MeshAdvModelAssetWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for MeshAdvModelAssetWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MeshAdvModelAssetWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for MeshAdvModelAssetWriter<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvModelAsset"
    }
}

impl<'a> MeshAdvModelAssetWriter<'a> {
    pub fn lods(self: &'a Self) -> DynamicArrayFieldWriter::<MeshAdvModelLodWriter> {
        DynamicArrayFieldWriter::<MeshAdvModelLodWriter>::new(self.0.push("lods"), &self.1)
    }
}
pub struct MeshAdvModelAssetOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for MeshAdvModelAssetOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        MeshAdvModelAssetOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for MeshAdvModelAssetOwned {
    fn schema_name() -> &'static str {
        "MeshAdvModelAsset"
    }
}

impl MeshAdvModelAssetOwned {
    pub fn lods(self: &Self) -> DynamicArrayFieldOwned::<MeshAdvModelLodOwned> {
        DynamicArrayFieldOwned::<MeshAdvModelLodOwned>::new(self.0.push("lods"), &self.1)
    }
}
#[derive(Default)]
pub struct MeshAdvModelLodAccessor(PropertyPath);

impl FieldAccessor for MeshAdvModelLodAccessor {
    fn new(property_path: PropertyPath) -> Self {
        MeshAdvModelLodAccessor(property_path)
    }
}

impl RecordAccessor for MeshAdvModelLodAccessor {
    fn schema_name() -> &'static str {
        "MeshAdvModelLod"
    }
}

impl MeshAdvModelLodAccessor {
    pub fn mesh(&self) -> AssetRefFieldAccessor {
        AssetRefFieldAccessor::new(self.0.push("mesh"))
    }
}
pub struct MeshAdvModelLodReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for MeshAdvModelLodReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MeshAdvModelLodReader(property_path, data_container)
    }
}

impl<'a> RecordReader for MeshAdvModelLodReader<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvModelLod"
    }
}

impl<'a> MeshAdvModelLodReader<'a> {
    pub fn mesh(&self) -> AssetRefFieldReader {
        AssetRefFieldReader::new(self.0.push("mesh"), self.1)
    }
}
pub struct MeshAdvModelLodWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for MeshAdvModelLodWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MeshAdvModelLodWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for MeshAdvModelLodWriter<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvModelLod"
    }
}

impl<'a> MeshAdvModelLodWriter<'a> {
    pub fn mesh(self: &'a Self) -> AssetRefFieldWriter {
        AssetRefFieldWriter::new(self.0.push("mesh"), &self.1)
    }
}
pub struct MeshAdvModelLodOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for MeshAdvModelLodOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        MeshAdvModelLodOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for MeshAdvModelLodOwned {
    fn schema_name() -> &'static str {
        "MeshAdvModelLod"
    }
}

impl MeshAdvModelLodOwned {
    pub fn mesh(self: &Self) -> AssetRefFieldOwned {
        AssetRefFieldOwned::new(self.0.push("mesh"), &self.1)
    }
}
#[derive(Default)]
pub struct MeshAdvPrefabAssetAccessor(PropertyPath);

impl FieldAccessor for MeshAdvPrefabAssetAccessor {
    fn new(property_path: PropertyPath) -> Self {
        MeshAdvPrefabAssetAccessor(property_path)
    }
}

impl RecordAccessor for MeshAdvPrefabAssetAccessor {
    fn schema_name() -> &'static str {
        "MeshAdvPrefabAsset"
    }
}

impl MeshAdvPrefabAssetAccessor {
}
pub struct MeshAdvPrefabAssetReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for MeshAdvPrefabAssetReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MeshAdvPrefabAssetReader(property_path, data_container)
    }
}

impl<'a> RecordReader for MeshAdvPrefabAssetReader<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvPrefabAsset"
    }
}

impl<'a> MeshAdvPrefabAssetReader<'a> {
}
pub struct MeshAdvPrefabAssetWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for MeshAdvPrefabAssetWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MeshAdvPrefabAssetWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for MeshAdvPrefabAssetWriter<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvPrefabAsset"
    }
}

impl<'a> MeshAdvPrefabAssetWriter<'a> {
}
pub struct MeshAdvPrefabAssetOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for MeshAdvPrefabAssetOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        MeshAdvPrefabAssetOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for MeshAdvPrefabAssetOwned {
    fn schema_name() -> &'static str {
        "MeshAdvPrefabAsset"
    }
}

impl MeshAdvPrefabAssetOwned {
}
#[derive(Default)]
pub struct MeshAdvPrefabImportDataAccessor(PropertyPath);

impl FieldAccessor for MeshAdvPrefabImportDataAccessor {
    fn new(property_path: PropertyPath) -> Self {
        MeshAdvPrefabImportDataAccessor(property_path)
    }
}

impl RecordAccessor for MeshAdvPrefabImportDataAccessor {
    fn schema_name() -> &'static str {
        "MeshAdvPrefabImportData"
    }
}

impl MeshAdvPrefabImportDataAccessor {
    pub fn json_data(&self) -> StringFieldAccessor {
        StringFieldAccessor::new(self.0.push("json_data"))
    }
}
pub struct MeshAdvPrefabImportDataReader<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldReader<'a> for MeshAdvPrefabImportDataReader<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MeshAdvPrefabImportDataReader(property_path, data_container)
    }
}

impl<'a> RecordReader for MeshAdvPrefabImportDataReader<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvPrefabImportData"
    }
}

impl<'a> MeshAdvPrefabImportDataReader<'a> {
    pub fn json_data(&self) -> StringFieldReader {
        StringFieldReader::new(self.0.push("json_data"), self.1)
    }
}
pub struct MeshAdvPrefabImportDataWriter<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldWriter<'a> for MeshAdvPrefabImportDataWriter<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MeshAdvPrefabImportDataWriter(property_path, data_container.clone())
    }
}

impl<'a> RecordWriter for MeshAdvPrefabImportDataWriter<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvPrefabImportData"
    }
}

impl<'a> MeshAdvPrefabImportDataWriter<'a> {
    pub fn json_data(self: &'a Self) -> StringFieldWriter {
        StringFieldWriter::new(self.0.push("json_data"), &self.1)
    }
}
pub struct MeshAdvPrefabImportDataOwned(PropertyPath, Rc<RefCell<Option<DataContainerOwned>>>);

impl FieldOwned for MeshAdvPrefabImportDataOwned {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainerOwned>>>) -> Self {
        MeshAdvPrefabImportDataOwned(property_path, data_container.clone())
    }
}

impl RecordOwned for MeshAdvPrefabImportDataOwned {
    fn schema_name() -> &'static str {
        "MeshAdvPrefabImportData"
    }
}

impl MeshAdvPrefabImportDataOwned {
    pub fn json_data(self: &Self) -> StringFieldOwned {
        StringFieldOwned::new(self.0.push("json_data"), &self.1)
    }
}
#[derive(Copy, Clone)]
pub enum MeshAdvShadowMethodEnum {
    None,
    Opaque,
}

impl Enum for MeshAdvShadowMethodEnum {
    fn to_symbol_name(&self) -> &'static str {
        match self {
            MeshAdvShadowMethodEnum::None => "None",
            MeshAdvShadowMethodEnum::Opaque => "Opaque",
        }
    }

    fn from_symbol_name(str: &str) -> Option<MeshAdvShadowMethodEnum> {
        match str {
            "None" => Some(MeshAdvShadowMethodEnum::None),
            "NONE" => Some(MeshAdvShadowMethodEnum::None),
            "Opaque" => Some(MeshAdvShadowMethodEnum::Opaque),
            "OPAQUE" => Some(MeshAdvShadowMethodEnum::Opaque),
            _ => None,
        }
    }
}

impl MeshAdvShadowMethodEnum {
    pub fn schema_name() -> &'static str {
        "MeshAdvShadowMethod"
    }
}
