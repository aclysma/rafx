// This file generated automatically by hydrate-codegen. Do not make manual edits. Use include!() to place these types in the intended location.
#[derive(Default)]
pub struct BlenderAnimAssetRecord(PropertyPath);

impl Field for BlenderAnimAssetRecord {
    fn new(property_path: PropertyPath) -> Self {
        BlenderAnimAssetRecord(property_path)
    }
}

impl Record for BlenderAnimAssetRecord {
    fn schema_name() -> &'static str {
        "BlenderAnimAsset"
    }
}

impl BlenderAnimAssetRecord {
}
#[derive(Default)]
pub struct BlenderAnimImportedDataRecord(PropertyPath);

impl Field for BlenderAnimImportedDataRecord {
    fn new(property_path: PropertyPath) -> Self {
        BlenderAnimImportedDataRecord(property_path)
    }
}

impl Record for BlenderAnimImportedDataRecord {
    fn schema_name() -> &'static str {
        "BlenderAnimImportedData"
    }
}

impl BlenderAnimImportedDataRecord {
    pub fn json_string(&self) -> StringField {
        StringField::new(self.0.push("json_string"))
    }
}
#[derive(Default)]
pub struct FontAssetRecord(PropertyPath);

impl Field for FontAssetRecord {
    fn new(property_path: PropertyPath) -> Self {
        FontAssetRecord(property_path)
    }
}

impl Record for FontAssetRecord {
    fn schema_name() -> &'static str {
        "FontAsset"
    }
}

impl FontAssetRecord {
}
#[derive(Default)]
pub struct FontImportedDataRecord(PropertyPath);

impl Field for FontImportedDataRecord {
    fn new(property_path: PropertyPath) -> Self {
        FontImportedDataRecord(property_path)
    }
}

impl Record for FontImportedDataRecord {
    fn schema_name() -> &'static str {
        "FontImportedData"
    }
}

impl FontImportedDataRecord {
    pub fn bytes(&self) -> BytesField {
        BytesField::new(self.0.push("bytes"))
    }
}
#[derive(Default)]
pub struct LdtkAssetRecord(PropertyPath);

impl Field for LdtkAssetRecord {
    fn new(property_path: PropertyPath) -> Self {
        LdtkAssetRecord(property_path)
    }
}

impl Record for LdtkAssetRecord {
    fn schema_name() -> &'static str {
        "LdtkAsset"
    }
}

impl LdtkAssetRecord {
}
#[derive(Default)]
pub struct LdtkImportDataRecord(PropertyPath);

impl Field for LdtkImportDataRecord {
    fn new(property_path: PropertyPath) -> Self {
        LdtkImportDataRecord(property_path)
    }
}

impl Record for LdtkImportDataRecord {
    fn schema_name() -> &'static str {
        "LdtkImportData"
    }
}

impl LdtkImportDataRecord {
    pub fn json_data(&self) -> StringField {
        StringField::new(self.0.push("json_data"))
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
pub struct MeshAdvMaterialAssetRecord(PropertyPath);

impl Field for MeshAdvMaterialAssetRecord {
    fn new(property_path: PropertyPath) -> Self {
        MeshAdvMaterialAssetRecord(property_path)
    }
}

impl Record for MeshAdvMaterialAssetRecord {
    fn schema_name() -> &'static str {
        "MeshAdvMaterialAsset"
    }
}

impl MeshAdvMaterialAssetRecord {
    pub fn alpha_threshold(&self) -> F32Field {
        F32Field::new(self.0.push("alpha_threshold"))
    }

    pub fn backface_culling(&self) -> BooleanField {
        BooleanField::new(self.0.push("backface_culling"))
    }

    pub fn base_color_factor(&self) -> Vec4Record {
        Vec4Record::new(self.0.push("base_color_factor"))
    }

    pub fn blend_method(&self) -> EnumField::<MeshAdvBlendMethodEnum> {
        EnumField::<MeshAdvBlendMethodEnum>::new(self.0.push("blend_method"))
    }

    pub fn color_texture(&self) -> ObjectRefField {
        ObjectRefField::new(self.0.push("color_texture"))
    }

    pub fn color_texture_has_alpha_channel(&self) -> BooleanField {
        BooleanField::new(self.0.push("color_texture_has_alpha_channel"))
    }

    pub fn emissive_factor(&self) -> Vec3Record {
        Vec3Record::new(self.0.push("emissive_factor"))
    }

    pub fn emissive_texture(&self) -> ObjectRefField {
        ObjectRefField::new(self.0.push("emissive_texture"))
    }

    pub fn metallic_factor(&self) -> F32Field {
        F32Field::new(self.0.push("metallic_factor"))
    }

    pub fn metallic_roughness_texture(&self) -> ObjectRefField {
        ObjectRefField::new(self.0.push("metallic_roughness_texture"))
    }

    pub fn normal_texture(&self) -> ObjectRefField {
        ObjectRefField::new(self.0.push("normal_texture"))
    }

    pub fn normal_texture_scale(&self) -> F32Field {
        F32Field::new(self.0.push("normal_texture_scale"))
    }

    pub fn occlusion_texture(&self) -> ObjectRefField {
        ObjectRefField::new(self.0.push("occlusion_texture"))
    }

    pub fn roughness_factor(&self) -> F32Field {
        F32Field::new(self.0.push("roughness_factor"))
    }

    pub fn shadow_method(&self) -> EnumField::<MeshAdvShadowMethodEnum> {
        EnumField::<MeshAdvShadowMethodEnum>::new(self.0.push("shadow_method"))
    }
}
#[derive(Default)]
pub struct MeshAdvMaterialImportedDataRecord(PropertyPath);

impl Field for MeshAdvMaterialImportedDataRecord {
    fn new(property_path: PropertyPath) -> Self {
        MeshAdvMaterialImportedDataRecord(property_path)
    }
}

impl Record for MeshAdvMaterialImportedDataRecord {
    fn schema_name() -> &'static str {
        "MeshAdvMaterialImportedData"
    }
}

impl MeshAdvMaterialImportedDataRecord {
}
#[derive(Default)]
pub struct MeshAdvMeshAssetRecord(PropertyPath);

impl Field for MeshAdvMeshAssetRecord {
    fn new(property_path: PropertyPath) -> Self {
        MeshAdvMeshAssetRecord(property_path)
    }
}

impl Record for MeshAdvMeshAssetRecord {
    fn schema_name() -> &'static str {
        "MeshAdvMeshAsset"
    }
}

impl MeshAdvMeshAssetRecord {
    pub fn material_slots(&self) -> DynamicArrayField::<ObjectRefField> {
        DynamicArrayField::<ObjectRefField>::new(self.0.push("material_slots"))
    }
}
#[derive(Default)]
pub struct MeshAdvMeshImportedDataRecord(PropertyPath);

impl Field for MeshAdvMeshImportedDataRecord {
    fn new(property_path: PropertyPath) -> Self {
        MeshAdvMeshImportedDataRecord(property_path)
    }
}

impl Record for MeshAdvMeshImportedDataRecord {
    fn schema_name() -> &'static str {
        "MeshAdvMeshImportedData"
    }
}

impl MeshAdvMeshImportedDataRecord {
    pub fn mesh_parts(&self) -> DynamicArrayField::<MeshAdvMeshImportedDataMeshPartRecord> {
        DynamicArrayField::<MeshAdvMeshImportedDataMeshPartRecord>::new(self.0.push("mesh_parts"))
    }
}
#[derive(Default)]
pub struct MeshAdvMeshImportedDataMeshPartRecord(PropertyPath);

impl Field for MeshAdvMeshImportedDataMeshPartRecord {
    fn new(property_path: PropertyPath) -> Self {
        MeshAdvMeshImportedDataMeshPartRecord(property_path)
    }
}

impl Record for MeshAdvMeshImportedDataMeshPartRecord {
    fn schema_name() -> &'static str {
        "MeshAdvMeshImportedDataMeshPart"
    }
}

impl MeshAdvMeshImportedDataMeshPartRecord {
    pub fn indices(&self) -> BytesField {
        BytesField::new(self.0.push("indices"))
    }

    pub fn material_index(&self) -> U32Field {
        U32Field::new(self.0.push("material_index"))
    }

    pub fn normals(&self) -> BytesField {
        BytesField::new(self.0.push("normals"))
    }

    pub fn positions(&self) -> BytesField {
        BytesField::new(self.0.push("positions"))
    }

    pub fn texture_coordinates(&self) -> BytesField {
        BytesField::new(self.0.push("texture_coordinates"))
    }
}
#[derive(Default)]
pub struct MeshAdvModelAssetRecord(PropertyPath);

impl Field for MeshAdvModelAssetRecord {
    fn new(property_path: PropertyPath) -> Self {
        MeshAdvModelAssetRecord(property_path)
    }
}

impl Record for MeshAdvModelAssetRecord {
    fn schema_name() -> &'static str {
        "MeshAdvModelAsset"
    }
}

impl MeshAdvModelAssetRecord {
    pub fn lods(&self) -> DynamicArrayField::<MeshAdvModelLodRecord> {
        DynamicArrayField::<MeshAdvModelLodRecord>::new(self.0.push("lods"))
    }
}
#[derive(Default)]
pub struct MeshAdvModelLodRecord(PropertyPath);

impl Field for MeshAdvModelLodRecord {
    fn new(property_path: PropertyPath) -> Self {
        MeshAdvModelLodRecord(property_path)
    }
}

impl Record for MeshAdvModelLodRecord {
    fn schema_name() -> &'static str {
        "MeshAdvModelLod"
    }
}

impl MeshAdvModelLodRecord {
    pub fn mesh(&self) -> ObjectRefField {
        ObjectRefField::new(self.0.push("mesh"))
    }
}
#[derive(Default)]
pub struct MeshAdvPrefabAssetRecord(PropertyPath);

impl Field for MeshAdvPrefabAssetRecord {
    fn new(property_path: PropertyPath) -> Self {
        MeshAdvPrefabAssetRecord(property_path)
    }
}

impl Record for MeshAdvPrefabAssetRecord {
    fn schema_name() -> &'static str {
        "MeshAdvPrefabAsset"
    }
}

impl MeshAdvPrefabAssetRecord {
}
#[derive(Default)]
pub struct MeshAdvPrefabImportDataRecord(PropertyPath);

impl Field for MeshAdvPrefabImportDataRecord {
    fn new(property_path: PropertyPath) -> Self {
        MeshAdvPrefabImportDataRecord(property_path)
    }
}

impl Record for MeshAdvPrefabImportDataRecord {
    fn schema_name() -> &'static str {
        "MeshAdvPrefabImportData"
    }
}

impl MeshAdvPrefabImportDataRecord {
    pub fn json_data(&self) -> StringField {
        StringField::new(self.0.push("json_data"))
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
