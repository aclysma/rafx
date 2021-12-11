use crate::assets::image::{
    ImageAssetColorSpaceConfig, ImageAssetData, ImageAssetDataFormatConfig,
};
use crate::distill::importer::ImportSource;
use crate::ImageAssetMipGeneration;
use distill::importer::{Error, ImportedAsset, Importer, ImporterValue};
use distill::{core::AssetUuid, importer::ImportOp};
use image::GenericImageView;
use rafx_api::RafxResourceType;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use type_uuid::*;

// Wrapper for image crate's supported formats
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ImageFileFormat {
    Png,
    Jpeg,
    Gif,
    WebP,
    Pnm,
    Tiff,
    Tga,
    Dds,
    Bmp,
    Ico,
    Hdr,
    Farbfeld,
    Avif,
}

impl From<image::ImageFormat> for ImageFileFormat {
    fn from(other: image::ImageFormat) -> ImageFileFormat {
        match other {
            image::ImageFormat::Png => ImageFileFormat::Png,
            image::ImageFormat::Jpeg => ImageFileFormat::Jpeg,
            image::ImageFormat::Gif => ImageFileFormat::Gif,
            image::ImageFormat::WebP => ImageFileFormat::WebP,
            image::ImageFormat::Pnm => ImageFileFormat::Pnm,
            image::ImageFormat::Tiff => ImageFileFormat::Tiff,
            image::ImageFormat::Tga => ImageFileFormat::Tga,
            image::ImageFormat::Dds => ImageFileFormat::Dds,
            image::ImageFormat::Bmp => ImageFileFormat::Bmp,
            image::ImageFormat::Ico => ImageFileFormat::Ico,
            image::ImageFormat::Hdr => ImageFileFormat::Hdr,
            image::ImageFormat::Farbfeld => ImageFileFormat::Farbfeld,
            image::ImageFormat::Avif => ImageFileFormat::Avif,
            _ => unimplemented!(),
        }
    }
}

impl Into<image::ImageFormat> for ImageFileFormat {
    fn into(self) -> image::ImageFormat {
        match self {
            ImageFileFormat::Png => image::ImageFormat::Png,
            ImageFileFormat::Jpeg => image::ImageFormat::Jpeg,
            ImageFileFormat::Gif => image::ImageFormat::Gif,
            ImageFileFormat::WebP => image::ImageFormat::WebP,
            ImageFileFormat::Pnm => image::ImageFormat::Pnm,
            ImageFileFormat::Tiff => image::ImageFormat::Tiff,
            ImageFileFormat::Tga => image::ImageFormat::Tga,
            ImageFileFormat::Dds => image::ImageFormat::Dds,
            ImageFileFormat::Bmp => image::ImageFormat::Bmp,
            ImageFileFormat::Ico => image::ImageFormat::Ico,
            ImageFileFormat::Hdr => image::ImageFormat::Hdr,
            ImageFileFormat::Farbfeld => image::ImageFormat::Farbfeld,
            ImageFileFormat::Avif => image::ImageFormat::Avif,
        }
    }
}

#[derive(TypeUuid, Serialize, Deserialize, Clone, Debug)]
#[uuid = "149d9973-6c02-4bcd-af6b-b7549aa92977"]
pub struct ImageImporterOptions {
    pub mip_generation: ImageAssetMipGeneration,
    pub color_space: ImageAssetColorSpaceConfig,
    pub data_format: ImageAssetDataFormatConfig,
}

impl Default for ImageImporterOptions {
    fn default() -> Self {
        ImageImporterOptions {
            mip_generation: ImageAssetMipGeneration::NoMips,
            color_space: ImageAssetColorSpaceConfig::Linear,
            data_format: ImageAssetDataFormatConfig::Uncompressed,
        }
    }
}

#[derive(Clone)]
pub struct ImageImporterRuleOptions {
    pub mip_generation: ImageAssetMipGeneration,
    pub color_space: ImageAssetColorSpaceConfig,
    pub data_format: ImageAssetDataFormatConfig,
}

pub trait ImageImporterRule: Send + Sync {
    fn try_apply(
        &self,
        path: &Path,
    ) -> Option<ImageImporterRuleOptions>;
}

pub struct ImageImporterRuleFilenameContains {
    pub search_string: String,
    pub rule_options: ImageImporterRuleOptions,
}

impl ImageImporterRule for ImageImporterRuleFilenameContains {
    fn try_apply(
        &self,
        path: &Path,
    ) -> Option<ImageImporterRuleOptions> {
        if let Some(file_name) = path.file_name() {
            if file_name
                .to_string_lossy()
                .to_lowercase()
                .contains(&self.search_string)
            {
                return Some(self.rule_options.clone());
            }
        }

        None
    }
}

pub struct ImageImporterConfig {
    default: ImageImporterRuleOptions,
    rules: Vec<Box<dyn ImageImporterRule>>,
}

impl ImageImporterConfig {
    pub fn new(default: ImageImporterRuleOptions) -> Self {
        ImageImporterConfig {
            default,
            rules: vec![],
        }
    }

    pub fn add_config(
        &mut self,
        rule: Box<dyn ImageImporterRule>,
    ) {
        self.rules.push(rule)
    }

    pub fn add_filename_contains_override<S: Into<String>>(
        &mut self,
        search_string: S,
        rule_options: ImageImporterRuleOptions,
    ) {
        self.add_config(Box::new(ImageImporterRuleFilenameContains {
            search_string: search_string.into().to_lowercase(),
            rule_options,
        }));
    }
}

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "23f90369-6916-4548-81d0-a76e0b162df2"]
pub struct ImageImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "4ae5ddc5-6805-4cf5-aa14-d44c6e0b8251"]
pub struct ImageImporter(pub ImageFileFormat, pub Arc<ImageImporterConfig>);
impl Importer for ImageImporter {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        7
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ImageImporterOptions;

    type State = ImageImporterState;

    fn default_options(
        &self,
        import_source: ImportSource,
    ) -> Option<Self::Options> {
        match import_source {
            ImportSource::File(path) => {
                for rule in &self.1.rules {
                    if let Some(options) = rule.try_apply(path) {
                        log::trace!("FOUND RULE FOR {:?}", path);
                        return Some(ImageImporterOptions {
                            mip_generation: options.mip_generation,
                            data_format: options.data_format,
                            color_space: options.color_space,
                        });
                    }
                }
            }
        }

        return Some(ImageImporterOptions {
            mip_generation: self.1.default.mip_generation,
            data_format: self.1.default.data_format,
            color_space: self.1.default.color_space,
        });
    }

    /// Reads the given bytes and produces assets.
    #[profiling::function]
    fn import(
        &self,
        _op: &mut ImportOp,
        source: &mut dyn Read,
        options: &Self::Options,
        state: &mut Self::State,
    ) -> distill::importer::Result<ImporterValue> {
        let id = state
            .0
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = ImageImporterState(Some(id));
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;

        use image::EncodableLayout;

        log::trace!("import with options {:?}", options);

        let decoded_image = image::load_from_memory_with_format(&bytes, self.0.into())
            .map_err(|e| Error::Boxed(Box::new(e)))?;
        let (width, height) = decoded_image.dimensions();
        let asset_data = ImageAssetData::from_raw_rgba32(
            width,
            height,
            options.color_space,
            options.data_format,
            options.mip_generation,
            RafxResourceType::TEXTURE,
            decoded_image.into_rgba8().as_bytes(),
        )
        .unwrap();

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(asset_data),
            }],
        })
    }
}
