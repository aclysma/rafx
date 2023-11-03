// This file generated automatically by hydrate-codegen. Do not make manual edits. Use include!() to place these types in the intended location.
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
