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
pub struct BlenderAnimAssetRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for BlenderAnimAssetRef<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        BlenderAnimAssetRef(property_path, data_container)
    }
}

impl<'a> RecordRef for BlenderAnimAssetRef<'a> {
    fn schema_name() -> &'static str {
        "BlenderAnimAsset"
    }
}

impl<'a> BlenderAnimAssetRef<'a> {
}
pub struct BlenderAnimAssetRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for BlenderAnimAssetRefMut<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        BlenderAnimAssetRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for BlenderAnimAssetRefMut<'a> {
    fn schema_name() -> &'static str {
        "BlenderAnimAsset"
    }
}

impl<'a> BlenderAnimAssetRefMut<'a> {
}
pub struct BlenderAnimAssetRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for BlenderAnimAssetRecord {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainer>>>) -> Self {
        BlenderAnimAssetRecord(property_path, data_container.clone())
    }
}

impl Record for BlenderAnimAssetRecord {
    type Reader<'a> = BlenderAnimAssetRef<'a>;
    type Writer<'a> = BlenderAnimAssetRefMut<'a>;
    type Accessor = BlenderAnimAssetAccessor;

    fn schema_name() -> &'static str {
        "BlenderAnimAsset"
    }
}

impl BlenderAnimAssetRecord {
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
pub struct BlenderAnimImportedDataRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for BlenderAnimImportedDataRef<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        BlenderAnimImportedDataRef(property_path, data_container)
    }
}

impl<'a> RecordRef for BlenderAnimImportedDataRef<'a> {
    fn schema_name() -> &'static str {
        "BlenderAnimImportedData"
    }
}

impl<'a> BlenderAnimImportedDataRef<'a> {
    pub fn json_string(&self) -> StringFieldRef {
        StringFieldRef::new(self.0.push("json_string"), self.1.clone())
    }
}
pub struct BlenderAnimImportedDataRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for BlenderAnimImportedDataRefMut<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        BlenderAnimImportedDataRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for BlenderAnimImportedDataRefMut<'a> {
    fn schema_name() -> &'static str {
        "BlenderAnimImportedData"
    }
}

impl<'a> BlenderAnimImportedDataRefMut<'a> {
    pub fn json_string(self: &'a Self) -> StringFieldRefMut {
        StringFieldRefMut::new(self.0.push("json_string"), &self.1)
    }
}
pub struct BlenderAnimImportedDataRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for BlenderAnimImportedDataRecord {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainer>>>) -> Self {
        BlenderAnimImportedDataRecord(property_path, data_container.clone())
    }
}

impl Record for BlenderAnimImportedDataRecord {
    type Reader<'a> = BlenderAnimImportedDataRef<'a>;
    type Writer<'a> = BlenderAnimImportedDataRefMut<'a>;
    type Accessor = BlenderAnimImportedDataAccessor;

    fn schema_name() -> &'static str {
        "BlenderAnimImportedData"
    }
}

impl BlenderAnimImportedDataRecord {
    pub fn json_string(self: &Self) -> StringField {
        StringField::new(self.0.push("json_string"), &self.1)
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
pub struct FontAssetRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for FontAssetRef<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        FontAssetRef(property_path, data_container)
    }
}

impl<'a> RecordRef for FontAssetRef<'a> {
    fn schema_name() -> &'static str {
        "FontAsset"
    }
}

impl<'a> FontAssetRef<'a> {
}
pub struct FontAssetRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for FontAssetRefMut<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        FontAssetRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for FontAssetRefMut<'a> {
    fn schema_name() -> &'static str {
        "FontAsset"
    }
}

impl<'a> FontAssetRefMut<'a> {
}
pub struct FontAssetRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for FontAssetRecord {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainer>>>) -> Self {
        FontAssetRecord(property_path, data_container.clone())
    }
}

impl Record for FontAssetRecord {
    type Reader<'a> = FontAssetRef<'a>;
    type Writer<'a> = FontAssetRefMut<'a>;
    type Accessor = FontAssetAccessor;

    fn schema_name() -> &'static str {
        "FontAsset"
    }
}

impl FontAssetRecord {
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
pub struct FontImportedDataRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for FontImportedDataRef<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        FontImportedDataRef(property_path, data_container)
    }
}

impl<'a> RecordRef for FontImportedDataRef<'a> {
    fn schema_name() -> &'static str {
        "FontImportedData"
    }
}

impl<'a> FontImportedDataRef<'a> {
    pub fn bytes(&self) -> BytesFieldRef {
        BytesFieldRef::new(self.0.push("bytes"), self.1.clone())
    }
}
pub struct FontImportedDataRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for FontImportedDataRefMut<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        FontImportedDataRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for FontImportedDataRefMut<'a> {
    fn schema_name() -> &'static str {
        "FontImportedData"
    }
}

impl<'a> FontImportedDataRefMut<'a> {
    pub fn bytes(self: &'a Self) -> BytesFieldRefMut {
        BytesFieldRefMut::new(self.0.push("bytes"), &self.1)
    }
}
pub struct FontImportedDataRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for FontImportedDataRecord {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainer>>>) -> Self {
        FontImportedDataRecord(property_path, data_container.clone())
    }
}

impl Record for FontImportedDataRecord {
    type Reader<'a> = FontImportedDataRef<'a>;
    type Writer<'a> = FontImportedDataRefMut<'a>;
    type Accessor = FontImportedDataAccessor;

    fn schema_name() -> &'static str {
        "FontImportedData"
    }
}

impl FontImportedDataRecord {
    pub fn bytes(self: &Self) -> BytesField {
        BytesField::new(self.0.push("bytes"), &self.1)
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
pub struct LdtkAssetRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for LdtkAssetRef<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        LdtkAssetRef(property_path, data_container)
    }
}

impl<'a> RecordRef for LdtkAssetRef<'a> {
    fn schema_name() -> &'static str {
        "LdtkAsset"
    }
}

impl<'a> LdtkAssetRef<'a> {
}
pub struct LdtkAssetRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for LdtkAssetRefMut<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        LdtkAssetRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for LdtkAssetRefMut<'a> {
    fn schema_name() -> &'static str {
        "LdtkAsset"
    }
}

impl<'a> LdtkAssetRefMut<'a> {
}
pub struct LdtkAssetRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for LdtkAssetRecord {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainer>>>) -> Self {
        LdtkAssetRecord(property_path, data_container.clone())
    }
}

impl Record for LdtkAssetRecord {
    type Reader<'a> = LdtkAssetRef<'a>;
    type Writer<'a> = LdtkAssetRefMut<'a>;
    type Accessor = LdtkAssetAccessor;

    fn schema_name() -> &'static str {
        "LdtkAsset"
    }
}

impl LdtkAssetRecord {
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
pub struct LdtkImportDataRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for LdtkImportDataRef<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        LdtkImportDataRef(property_path, data_container)
    }
}

impl<'a> RecordRef for LdtkImportDataRef<'a> {
    fn schema_name() -> &'static str {
        "LdtkImportData"
    }
}

impl<'a> LdtkImportDataRef<'a> {
    pub fn json_data(&self) -> StringFieldRef {
        StringFieldRef::new(self.0.push("json_data"), self.1.clone())
    }
}
pub struct LdtkImportDataRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for LdtkImportDataRefMut<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        LdtkImportDataRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for LdtkImportDataRefMut<'a> {
    fn schema_name() -> &'static str {
        "LdtkImportData"
    }
}

impl<'a> LdtkImportDataRefMut<'a> {
    pub fn json_data(self: &'a Self) -> StringFieldRefMut {
        StringFieldRefMut::new(self.0.push("json_data"), &self.1)
    }
}
pub struct LdtkImportDataRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for LdtkImportDataRecord {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainer>>>) -> Self {
        LdtkImportDataRecord(property_path, data_container.clone())
    }
}

impl Record for LdtkImportDataRecord {
    type Reader<'a> = LdtkImportDataRef<'a>;
    type Writer<'a> = LdtkImportDataRefMut<'a>;
    type Accessor = LdtkImportDataAccessor;

    fn schema_name() -> &'static str {
        "LdtkImportData"
    }
}

impl LdtkImportDataRecord {
    pub fn json_data(self: &Self) -> StringField {
        StringField::new(self.0.push("json_data"), &self.1)
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
pub struct MeshAdvMaterialAssetRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for MeshAdvMaterialAssetRef<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MeshAdvMaterialAssetRef(property_path, data_container)
    }
}

impl<'a> RecordRef for MeshAdvMaterialAssetRef<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvMaterialAsset"
    }
}

impl<'a> MeshAdvMaterialAssetRef<'a> {
    pub fn alpha_threshold(&self) -> F32FieldRef {
        F32FieldRef::new(self.0.push("alpha_threshold"), self.1.clone())
    }

    pub fn backface_culling(&self) -> BooleanFieldRef {
        BooleanFieldRef::new(self.0.push("backface_culling"), self.1.clone())
    }

    pub fn base_color_factor(&self) -> Vec4Ref {
        Vec4Ref::new(self.0.push("base_color_factor"), self.1.clone())
    }

    pub fn blend_method(&self) -> EnumFieldRef::<MeshAdvBlendMethodEnum> {
        EnumFieldRef::<MeshAdvBlendMethodEnum>::new(self.0.push("blend_method"), self.1.clone())
    }

    pub fn color_texture(&self) -> AssetRefFieldRef {
        AssetRefFieldRef::new(self.0.push("color_texture"), self.1.clone())
    }

    pub fn color_texture_has_alpha_channel(&self) -> BooleanFieldRef {
        BooleanFieldRef::new(self.0.push("color_texture_has_alpha_channel"), self.1.clone())
    }

    pub fn emissive_factor(&self) -> Vec3Ref {
        Vec3Ref::new(self.0.push("emissive_factor"), self.1.clone())
    }

    pub fn emissive_texture(&self) -> AssetRefFieldRef {
        AssetRefFieldRef::new(self.0.push("emissive_texture"), self.1.clone())
    }

    pub fn metallic_factor(&self) -> F32FieldRef {
        F32FieldRef::new(self.0.push("metallic_factor"), self.1.clone())
    }

    pub fn metallic_roughness_texture(&self) -> AssetRefFieldRef {
        AssetRefFieldRef::new(self.0.push("metallic_roughness_texture"), self.1.clone())
    }

    pub fn normal_texture(&self) -> AssetRefFieldRef {
        AssetRefFieldRef::new(self.0.push("normal_texture"), self.1.clone())
    }

    pub fn normal_texture_scale(&self) -> F32FieldRef {
        F32FieldRef::new(self.0.push("normal_texture_scale"), self.1.clone())
    }

    pub fn occlusion_texture(&self) -> AssetRefFieldRef {
        AssetRefFieldRef::new(self.0.push("occlusion_texture"), self.1.clone())
    }

    pub fn roughness_factor(&self) -> F32FieldRef {
        F32FieldRef::new(self.0.push("roughness_factor"), self.1.clone())
    }

    pub fn shadow_method(&self) -> EnumFieldRef::<MeshAdvShadowMethodEnum> {
        EnumFieldRef::<MeshAdvShadowMethodEnum>::new(self.0.push("shadow_method"), self.1.clone())
    }
}
pub struct MeshAdvMaterialAssetRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for MeshAdvMaterialAssetRefMut<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MeshAdvMaterialAssetRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for MeshAdvMaterialAssetRefMut<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvMaterialAsset"
    }
}

impl<'a> MeshAdvMaterialAssetRefMut<'a> {
    pub fn alpha_threshold(self: &'a Self) -> F32FieldRefMut {
        F32FieldRefMut::new(self.0.push("alpha_threshold"), &self.1)
    }

    pub fn backface_culling(self: &'a Self) -> BooleanFieldRefMut {
        BooleanFieldRefMut::new(self.0.push("backface_culling"), &self.1)
    }

    pub fn base_color_factor(self: &'a Self) -> Vec4RefMut {
        Vec4RefMut::new(self.0.push("base_color_factor"), &self.1)
    }

    pub fn blend_method(self: &'a Self) -> EnumFieldRefMut::<MeshAdvBlendMethodEnum> {
        EnumFieldRefMut::<MeshAdvBlendMethodEnum>::new(self.0.push("blend_method"), &self.1)
    }

    pub fn color_texture(self: &'a Self) -> AssetRefFieldRefMut {
        AssetRefFieldRefMut::new(self.0.push("color_texture"), &self.1)
    }

    pub fn color_texture_has_alpha_channel(self: &'a Self) -> BooleanFieldRefMut {
        BooleanFieldRefMut::new(self.0.push("color_texture_has_alpha_channel"), &self.1)
    }

    pub fn emissive_factor(self: &'a Self) -> Vec3RefMut {
        Vec3RefMut::new(self.0.push("emissive_factor"), &self.1)
    }

    pub fn emissive_texture(self: &'a Self) -> AssetRefFieldRefMut {
        AssetRefFieldRefMut::new(self.0.push("emissive_texture"), &self.1)
    }

    pub fn metallic_factor(self: &'a Self) -> F32FieldRefMut {
        F32FieldRefMut::new(self.0.push("metallic_factor"), &self.1)
    }

    pub fn metallic_roughness_texture(self: &'a Self) -> AssetRefFieldRefMut {
        AssetRefFieldRefMut::new(self.0.push("metallic_roughness_texture"), &self.1)
    }

    pub fn normal_texture(self: &'a Self) -> AssetRefFieldRefMut {
        AssetRefFieldRefMut::new(self.0.push("normal_texture"), &self.1)
    }

    pub fn normal_texture_scale(self: &'a Self) -> F32FieldRefMut {
        F32FieldRefMut::new(self.0.push("normal_texture_scale"), &self.1)
    }

    pub fn occlusion_texture(self: &'a Self) -> AssetRefFieldRefMut {
        AssetRefFieldRefMut::new(self.0.push("occlusion_texture"), &self.1)
    }

    pub fn roughness_factor(self: &'a Self) -> F32FieldRefMut {
        F32FieldRefMut::new(self.0.push("roughness_factor"), &self.1)
    }

    pub fn shadow_method(self: &'a Self) -> EnumFieldRefMut::<MeshAdvShadowMethodEnum> {
        EnumFieldRefMut::<MeshAdvShadowMethodEnum>::new(self.0.push("shadow_method"), &self.1)
    }
}
pub struct MeshAdvMaterialAssetRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for MeshAdvMaterialAssetRecord {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainer>>>) -> Self {
        MeshAdvMaterialAssetRecord(property_path, data_container.clone())
    }
}

impl Record for MeshAdvMaterialAssetRecord {
    type Reader<'a> = MeshAdvMaterialAssetRef<'a>;
    type Writer<'a> = MeshAdvMaterialAssetRefMut<'a>;
    type Accessor = MeshAdvMaterialAssetAccessor;

    fn schema_name() -> &'static str {
        "MeshAdvMaterialAsset"
    }
}

impl MeshAdvMaterialAssetRecord {
    pub fn alpha_threshold(self: &Self) -> F32Field {
        F32Field::new(self.0.push("alpha_threshold"), &self.1)
    }

    pub fn backface_culling(self: &Self) -> BooleanField {
        BooleanField::new(self.0.push("backface_culling"), &self.1)
    }

    pub fn base_color_factor(self: &Self) -> Vec4Record {
        Vec4Record::new(self.0.push("base_color_factor"), &self.1)
    }

    pub fn blend_method(self: &Self) -> EnumField::<MeshAdvBlendMethodEnum> {
        EnumField::<MeshAdvBlendMethodEnum>::new(self.0.push("blend_method"), &self.1)
    }

    pub fn color_texture(self: &Self) -> AssetRefField {
        AssetRefField::new(self.0.push("color_texture"), &self.1)
    }

    pub fn color_texture_has_alpha_channel(self: &Self) -> BooleanField {
        BooleanField::new(self.0.push("color_texture_has_alpha_channel"), &self.1)
    }

    pub fn emissive_factor(self: &Self) -> Vec3Record {
        Vec3Record::new(self.0.push("emissive_factor"), &self.1)
    }

    pub fn emissive_texture(self: &Self) -> AssetRefField {
        AssetRefField::new(self.0.push("emissive_texture"), &self.1)
    }

    pub fn metallic_factor(self: &Self) -> F32Field {
        F32Field::new(self.0.push("metallic_factor"), &self.1)
    }

    pub fn metallic_roughness_texture(self: &Self) -> AssetRefField {
        AssetRefField::new(self.0.push("metallic_roughness_texture"), &self.1)
    }

    pub fn normal_texture(self: &Self) -> AssetRefField {
        AssetRefField::new(self.0.push("normal_texture"), &self.1)
    }

    pub fn normal_texture_scale(self: &Self) -> F32Field {
        F32Field::new(self.0.push("normal_texture_scale"), &self.1)
    }

    pub fn occlusion_texture(self: &Self) -> AssetRefField {
        AssetRefField::new(self.0.push("occlusion_texture"), &self.1)
    }

    pub fn roughness_factor(self: &Self) -> F32Field {
        F32Field::new(self.0.push("roughness_factor"), &self.1)
    }

    pub fn shadow_method(self: &Self) -> EnumField::<MeshAdvShadowMethodEnum> {
        EnumField::<MeshAdvShadowMethodEnum>::new(self.0.push("shadow_method"), &self.1)
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
pub struct MeshAdvMaterialImportedDataRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for MeshAdvMaterialImportedDataRef<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MeshAdvMaterialImportedDataRef(property_path, data_container)
    }
}

impl<'a> RecordRef for MeshAdvMaterialImportedDataRef<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvMaterialImportedData"
    }
}

impl<'a> MeshAdvMaterialImportedDataRef<'a> {
}
pub struct MeshAdvMaterialImportedDataRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for MeshAdvMaterialImportedDataRefMut<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MeshAdvMaterialImportedDataRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for MeshAdvMaterialImportedDataRefMut<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvMaterialImportedData"
    }
}

impl<'a> MeshAdvMaterialImportedDataRefMut<'a> {
}
pub struct MeshAdvMaterialImportedDataRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for MeshAdvMaterialImportedDataRecord {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainer>>>) -> Self {
        MeshAdvMaterialImportedDataRecord(property_path, data_container.clone())
    }
}

impl Record for MeshAdvMaterialImportedDataRecord {
    type Reader<'a> = MeshAdvMaterialImportedDataRef<'a>;
    type Writer<'a> = MeshAdvMaterialImportedDataRefMut<'a>;
    type Accessor = MeshAdvMaterialImportedDataAccessor;

    fn schema_name() -> &'static str {
        "MeshAdvMaterialImportedData"
    }
}

impl MeshAdvMaterialImportedDataRecord {
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
pub struct MeshAdvMeshAssetRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for MeshAdvMeshAssetRef<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MeshAdvMeshAssetRef(property_path, data_container)
    }
}

impl<'a> RecordRef for MeshAdvMeshAssetRef<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvMeshAsset"
    }
}

impl<'a> MeshAdvMeshAssetRef<'a> {
    pub fn material_slots(&self) -> DynamicArrayFieldRef::<AssetRefFieldRef> {
        DynamicArrayFieldRef::<AssetRefFieldRef>::new(self.0.push("material_slots"), self.1.clone())
    }
}
pub struct MeshAdvMeshAssetRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for MeshAdvMeshAssetRefMut<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MeshAdvMeshAssetRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for MeshAdvMeshAssetRefMut<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvMeshAsset"
    }
}

impl<'a> MeshAdvMeshAssetRefMut<'a> {
    pub fn material_slots(self: &'a Self) -> DynamicArrayFieldRefMut::<AssetRefFieldRefMut> {
        DynamicArrayFieldRefMut::<AssetRefFieldRefMut>::new(self.0.push("material_slots"), &self.1)
    }
}
pub struct MeshAdvMeshAssetRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for MeshAdvMeshAssetRecord {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainer>>>) -> Self {
        MeshAdvMeshAssetRecord(property_path, data_container.clone())
    }
}

impl Record for MeshAdvMeshAssetRecord {
    type Reader<'a> = MeshAdvMeshAssetRef<'a>;
    type Writer<'a> = MeshAdvMeshAssetRefMut<'a>;
    type Accessor = MeshAdvMeshAssetAccessor;

    fn schema_name() -> &'static str {
        "MeshAdvMeshAsset"
    }
}

impl MeshAdvMeshAssetRecord {
    pub fn material_slots(self: &Self) -> DynamicArrayField::<AssetRefField> {
        DynamicArrayField::<AssetRefField>::new(self.0.push("material_slots"), &self.1)
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
pub struct MeshAdvMeshImportedDataRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for MeshAdvMeshImportedDataRef<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MeshAdvMeshImportedDataRef(property_path, data_container)
    }
}

impl<'a> RecordRef for MeshAdvMeshImportedDataRef<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvMeshImportedData"
    }
}

impl<'a> MeshAdvMeshImportedDataRef<'a> {
    pub fn mesh_parts(&self) -> DynamicArrayFieldRef::<MeshAdvMeshImportedDataMeshPartRef> {
        DynamicArrayFieldRef::<MeshAdvMeshImportedDataMeshPartRef>::new(self.0.push("mesh_parts"), self.1.clone())
    }
}
pub struct MeshAdvMeshImportedDataRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for MeshAdvMeshImportedDataRefMut<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MeshAdvMeshImportedDataRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for MeshAdvMeshImportedDataRefMut<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvMeshImportedData"
    }
}

impl<'a> MeshAdvMeshImportedDataRefMut<'a> {
    pub fn mesh_parts(self: &'a Self) -> DynamicArrayFieldRefMut::<MeshAdvMeshImportedDataMeshPartRefMut> {
        DynamicArrayFieldRefMut::<MeshAdvMeshImportedDataMeshPartRefMut>::new(self.0.push("mesh_parts"), &self.1)
    }
}
pub struct MeshAdvMeshImportedDataRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for MeshAdvMeshImportedDataRecord {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainer>>>) -> Self {
        MeshAdvMeshImportedDataRecord(property_path, data_container.clone())
    }
}

impl Record for MeshAdvMeshImportedDataRecord {
    type Reader<'a> = MeshAdvMeshImportedDataRef<'a>;
    type Writer<'a> = MeshAdvMeshImportedDataRefMut<'a>;
    type Accessor = MeshAdvMeshImportedDataAccessor;

    fn schema_name() -> &'static str {
        "MeshAdvMeshImportedData"
    }
}

impl MeshAdvMeshImportedDataRecord {
    pub fn mesh_parts(self: &Self) -> DynamicArrayField::<MeshAdvMeshImportedDataMeshPartRecord> {
        DynamicArrayField::<MeshAdvMeshImportedDataMeshPartRecord>::new(self.0.push("mesh_parts"), &self.1)
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
pub struct MeshAdvMeshImportedDataMeshPartRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for MeshAdvMeshImportedDataMeshPartRef<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MeshAdvMeshImportedDataMeshPartRef(property_path, data_container)
    }
}

impl<'a> RecordRef for MeshAdvMeshImportedDataMeshPartRef<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvMeshImportedDataMeshPart"
    }
}

impl<'a> MeshAdvMeshImportedDataMeshPartRef<'a> {
    pub fn indices(&self) -> BytesFieldRef {
        BytesFieldRef::new(self.0.push("indices"), self.1.clone())
    }

    pub fn material_index(&self) -> U32FieldRef {
        U32FieldRef::new(self.0.push("material_index"), self.1.clone())
    }

    pub fn normals(&self) -> BytesFieldRef {
        BytesFieldRef::new(self.0.push("normals"), self.1.clone())
    }

    pub fn positions(&self) -> BytesFieldRef {
        BytesFieldRef::new(self.0.push("positions"), self.1.clone())
    }

    pub fn texture_coordinates(&self) -> BytesFieldRef {
        BytesFieldRef::new(self.0.push("texture_coordinates"), self.1.clone())
    }
}
pub struct MeshAdvMeshImportedDataMeshPartRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for MeshAdvMeshImportedDataMeshPartRefMut<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MeshAdvMeshImportedDataMeshPartRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for MeshAdvMeshImportedDataMeshPartRefMut<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvMeshImportedDataMeshPart"
    }
}

impl<'a> MeshAdvMeshImportedDataMeshPartRefMut<'a> {
    pub fn indices(self: &'a Self) -> BytesFieldRefMut {
        BytesFieldRefMut::new(self.0.push("indices"), &self.1)
    }

    pub fn material_index(self: &'a Self) -> U32FieldRefMut {
        U32FieldRefMut::new(self.0.push("material_index"), &self.1)
    }

    pub fn normals(self: &'a Self) -> BytesFieldRefMut {
        BytesFieldRefMut::new(self.0.push("normals"), &self.1)
    }

    pub fn positions(self: &'a Self) -> BytesFieldRefMut {
        BytesFieldRefMut::new(self.0.push("positions"), &self.1)
    }

    pub fn texture_coordinates(self: &'a Self) -> BytesFieldRefMut {
        BytesFieldRefMut::new(self.0.push("texture_coordinates"), &self.1)
    }
}
pub struct MeshAdvMeshImportedDataMeshPartRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for MeshAdvMeshImportedDataMeshPartRecord {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainer>>>) -> Self {
        MeshAdvMeshImportedDataMeshPartRecord(property_path, data_container.clone())
    }
}

impl Record for MeshAdvMeshImportedDataMeshPartRecord {
    type Reader<'a> = MeshAdvMeshImportedDataMeshPartRef<'a>;
    type Writer<'a> = MeshAdvMeshImportedDataMeshPartRefMut<'a>;
    type Accessor = MeshAdvMeshImportedDataMeshPartAccessor;

    fn schema_name() -> &'static str {
        "MeshAdvMeshImportedDataMeshPart"
    }
}

impl MeshAdvMeshImportedDataMeshPartRecord {
    pub fn indices(self: &Self) -> BytesField {
        BytesField::new(self.0.push("indices"), &self.1)
    }

    pub fn material_index(self: &Self) -> U32Field {
        U32Field::new(self.0.push("material_index"), &self.1)
    }

    pub fn normals(self: &Self) -> BytesField {
        BytesField::new(self.0.push("normals"), &self.1)
    }

    pub fn positions(self: &Self) -> BytesField {
        BytesField::new(self.0.push("positions"), &self.1)
    }

    pub fn texture_coordinates(self: &Self) -> BytesField {
        BytesField::new(self.0.push("texture_coordinates"), &self.1)
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
pub struct MeshAdvModelAssetRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for MeshAdvModelAssetRef<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MeshAdvModelAssetRef(property_path, data_container)
    }
}

impl<'a> RecordRef for MeshAdvModelAssetRef<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvModelAsset"
    }
}

impl<'a> MeshAdvModelAssetRef<'a> {
    pub fn lods(&self) -> DynamicArrayFieldRef::<MeshAdvModelLodRef> {
        DynamicArrayFieldRef::<MeshAdvModelLodRef>::new(self.0.push("lods"), self.1.clone())
    }
}
pub struct MeshAdvModelAssetRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for MeshAdvModelAssetRefMut<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MeshAdvModelAssetRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for MeshAdvModelAssetRefMut<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvModelAsset"
    }
}

impl<'a> MeshAdvModelAssetRefMut<'a> {
    pub fn lods(self: &'a Self) -> DynamicArrayFieldRefMut::<MeshAdvModelLodRefMut> {
        DynamicArrayFieldRefMut::<MeshAdvModelLodRefMut>::new(self.0.push("lods"), &self.1)
    }
}
pub struct MeshAdvModelAssetRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for MeshAdvModelAssetRecord {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainer>>>) -> Self {
        MeshAdvModelAssetRecord(property_path, data_container.clone())
    }
}

impl Record for MeshAdvModelAssetRecord {
    type Reader<'a> = MeshAdvModelAssetRef<'a>;
    type Writer<'a> = MeshAdvModelAssetRefMut<'a>;
    type Accessor = MeshAdvModelAssetAccessor;

    fn schema_name() -> &'static str {
        "MeshAdvModelAsset"
    }
}

impl MeshAdvModelAssetRecord {
    pub fn lods(self: &Self) -> DynamicArrayField::<MeshAdvModelLodRecord> {
        DynamicArrayField::<MeshAdvModelLodRecord>::new(self.0.push("lods"), &self.1)
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
pub struct MeshAdvModelLodRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for MeshAdvModelLodRef<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MeshAdvModelLodRef(property_path, data_container)
    }
}

impl<'a> RecordRef for MeshAdvModelLodRef<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvModelLod"
    }
}

impl<'a> MeshAdvModelLodRef<'a> {
    pub fn mesh(&self) -> AssetRefFieldRef {
        AssetRefFieldRef::new(self.0.push("mesh"), self.1.clone())
    }
}
pub struct MeshAdvModelLodRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for MeshAdvModelLodRefMut<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MeshAdvModelLodRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for MeshAdvModelLodRefMut<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvModelLod"
    }
}

impl<'a> MeshAdvModelLodRefMut<'a> {
    pub fn mesh(self: &'a Self) -> AssetRefFieldRefMut {
        AssetRefFieldRefMut::new(self.0.push("mesh"), &self.1)
    }
}
pub struct MeshAdvModelLodRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for MeshAdvModelLodRecord {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainer>>>) -> Self {
        MeshAdvModelLodRecord(property_path, data_container.clone())
    }
}

impl Record for MeshAdvModelLodRecord {
    type Reader<'a> = MeshAdvModelLodRef<'a>;
    type Writer<'a> = MeshAdvModelLodRefMut<'a>;
    type Accessor = MeshAdvModelLodAccessor;

    fn schema_name() -> &'static str {
        "MeshAdvModelLod"
    }
}

impl MeshAdvModelLodRecord {
    pub fn mesh(self: &Self) -> AssetRefField {
        AssetRefField::new(self.0.push("mesh"), &self.1)
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
pub struct MeshAdvPrefabAssetRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for MeshAdvPrefabAssetRef<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MeshAdvPrefabAssetRef(property_path, data_container)
    }
}

impl<'a> RecordRef for MeshAdvPrefabAssetRef<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvPrefabAsset"
    }
}

impl<'a> MeshAdvPrefabAssetRef<'a> {
}
pub struct MeshAdvPrefabAssetRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for MeshAdvPrefabAssetRefMut<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MeshAdvPrefabAssetRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for MeshAdvPrefabAssetRefMut<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvPrefabAsset"
    }
}

impl<'a> MeshAdvPrefabAssetRefMut<'a> {
}
pub struct MeshAdvPrefabAssetRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for MeshAdvPrefabAssetRecord {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainer>>>) -> Self {
        MeshAdvPrefabAssetRecord(property_path, data_container.clone())
    }
}

impl Record for MeshAdvPrefabAssetRecord {
    type Reader<'a> = MeshAdvPrefabAssetRef<'a>;
    type Writer<'a> = MeshAdvPrefabAssetRefMut<'a>;
    type Accessor = MeshAdvPrefabAssetAccessor;

    fn schema_name() -> &'static str {
        "MeshAdvPrefabAsset"
    }
}

impl MeshAdvPrefabAssetRecord {
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
pub struct MeshAdvPrefabImportDataRef<'a>(PropertyPath, DataContainerRef<'a>);

impl<'a> FieldRef<'a> for MeshAdvPrefabImportDataRef<'a> {
    fn new(property_path: PropertyPath, data_container: DataContainerRef<'a>) -> Self {
        MeshAdvPrefabImportDataRef(property_path, data_container)
    }
}

impl<'a> RecordRef for MeshAdvPrefabImportDataRef<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvPrefabImportData"
    }
}

impl<'a> MeshAdvPrefabImportDataRef<'a> {
    pub fn json_data(&self) -> StringFieldRef {
        StringFieldRef::new(self.0.push("json_data"), self.1.clone())
    }
}
pub struct MeshAdvPrefabImportDataRefMut<'a>(PropertyPath, Rc<RefCell<DataContainerRefMut<'a>>>);

impl<'a> FieldRefMut<'a> for MeshAdvPrefabImportDataRefMut<'a> {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<DataContainerRefMut<'a>>>) -> Self {
        MeshAdvPrefabImportDataRefMut(property_path, data_container.clone())
    }
}

impl<'a> RecordRefMut for MeshAdvPrefabImportDataRefMut<'a> {
    fn schema_name() -> &'static str {
        "MeshAdvPrefabImportData"
    }
}

impl<'a> MeshAdvPrefabImportDataRefMut<'a> {
    pub fn json_data(self: &'a Self) -> StringFieldRefMut {
        StringFieldRefMut::new(self.0.push("json_data"), &self.1)
    }
}
pub struct MeshAdvPrefabImportDataRecord(PropertyPath, Rc<RefCell<Option<DataContainer>>>);

impl Field for MeshAdvPrefabImportDataRecord {
    fn new(property_path: PropertyPath, data_container: &Rc<RefCell<Option<DataContainer>>>) -> Self {
        MeshAdvPrefabImportDataRecord(property_path, data_container.clone())
    }
}

impl Record for MeshAdvPrefabImportDataRecord {
    type Reader<'a> = MeshAdvPrefabImportDataRef<'a>;
    type Writer<'a> = MeshAdvPrefabImportDataRefMut<'a>;
    type Accessor = MeshAdvPrefabImportDataAccessor;

    fn schema_name() -> &'static str {
        "MeshAdvPrefabImportData"
    }
}

impl MeshAdvPrefabImportDataRecord {
    pub fn json_data(self: &Self) -> StringField {
        StringField::new(self.0.push("json_data"), &self.1)
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
