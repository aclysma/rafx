use crate::assets::image::{
    ImageAssetColorSpaceConfig, ImageAssetData, ImageAssetDataFormatConfig,
};
use crate::schema::{
    GpuImageAssetAccessor, GpuImageAssetRecord, GpuImageBasisCompressionTypeEnum,
    GpuImageColorSpaceEnum, GpuImageImportedDataRecord, GpuImageMipGenerationEnum,
};
use crate::{
    ImageAssetBasisCompressionSettings, ImageAssetBasisCompressionType, ImageAssetMipGeneration,
};
use hydrate_base::AssetId;
use hydrate_data::{Record, RecordAccessor, RecordBuilder};
use hydrate_pipeline::{
    AssetPlugin, AssetPluginSetupContext, Builder, BuilderContext, BuilderRegistryBuilder,
    ImportContext, Importer, ImporterRegistryBuilder, JobInput, JobOutput, JobProcessor,
    JobProcessorRegistryBuilder, PipelineResult, RunContext, ScanContext, ThumbnailImage,
    ThumbnailProvider, ThumbnailProviderGatherContext, ThumbnailProviderRenderContext,
};
use image::{GenericImageView, Pixel};
use rafx_api::RafxResourceType;
use serde::{Deserialize, Serialize};
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

#[derive(TypeUuid)]
#[uuid = "e7c83acb-f73b-4b3c-b14d-fe5cc17c0fa3"]
pub struct GpuImageImporterSimple {
    pub image_importer_config: Arc<ImageImporterConfig>,
}

impl GpuImageImporterSimple {
    fn default_settings(
        &self,
        path: &Path,
    ) -> ImageImporterOptions {
        for rule in &self.image_importer_config.rules {
            if let Some(options) = rule.try_apply(path) {
                log::trace!("FOUND RULE FOR {:?}", path);
                return ImageImporterOptions {
                    mip_generation: options.mip_generation,
                    data_format: options.data_format,
                    color_space: options.color_space,
                };
            }
        }
        return ImageImporterOptions {
            mip_generation: self.image_importer_config.default.mip_generation,
            data_format: self.image_importer_config.default.data_format,
            color_space: self.image_importer_config.default.color_space,
        };
    }

    pub fn set_default_asset_properties(
        default_settings: &ImageImporterOptions,
        asset_record: &mut RecordBuilder<GpuImageAssetRecord>,
    ) {
        match default_settings.data_format {
            ImageAssetDataFormatConfig::Uncompressed => {
                asset_record.basis_compression().set(false).unwrap();
            }
            ImageAssetDataFormatConfig::BasisCompressed(settings) => {
                asset_record.basis_compression().set(true).unwrap();

                asset_record
                    .basis_compression_settings()
                    .compression_type()
                    .set(match settings.compression_type {
                        ImageAssetBasisCompressionType::Etc1S => {
                            GpuImageBasisCompressionTypeEnum::Etc1S
                        }
                        ImageAssetBasisCompressionType::Uastc => {
                            GpuImageBasisCompressionTypeEnum::Uastc
                        }
                    })
                    .unwrap();
                asset_record
                    .basis_compression_settings()
                    .quality()
                    .set(settings.quality)
                    .unwrap();
            }
        }
        asset_record
            .color_space()
            .set(match default_settings.color_space {
                ImageAssetColorSpaceConfig::Srgb => GpuImageColorSpaceEnum::Srgb,
                ImageAssetColorSpaceConfig::Linear => GpuImageColorSpaceEnum::Linear,
            })
            .unwrap();
        asset_record
            .mip_generation()
            .set(match default_settings.mip_generation {
                ImageAssetMipGeneration::NoMips => GpuImageMipGenerationEnum::NoMips,
                ImageAssetMipGeneration::Precomupted => GpuImageMipGenerationEnum::Precomputed,
                ImageAssetMipGeneration::Runtime => GpuImageMipGenerationEnum::Runtime,
            })
            .unwrap();
    }
}

impl Importer for GpuImageImporterSimple {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["png", "jpg", "jpeg", "tga", "tif", "tiff", "bmp"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> PipelineResult<()> {
        context.add_default_importable::<GpuImageAssetRecord>()?;
        Ok(())
    }

    fn import_file(
        &self,
        context: ImportContext,
    ) -> PipelineResult<()> {
        let (image_bytes, width, height) = {
            profiling::scope!("Load Image from Disk");
            let decoded_image = ::image::open(context.path).unwrap();
            let (width, height) = decoded_image.dimensions();
            let image_bytes = decoded_image.into_rgba8().to_vec();
            (image_bytes, width, height)
        };

        //
        // Create import data
        //
        let import_data = GpuImageImportedDataRecord::new_builder(context.schema_set);
        import_data
            .image_bytes()
            .set(Arc::new(image_bytes))
            .unwrap();
        import_data.width().set(width).unwrap();
        import_data.height().set(height).unwrap();

        //
        // Create the default asset
        //
        let mut default_asset = GpuImageAssetRecord::new_builder(context.schema_set);
        let default_settings = self.default_settings(context.path);

        GpuImageImporterSimple::set_default_asset_properties(&default_settings, &mut default_asset);

        //
        // Return the created objects
        //
        context
            .add_default_importable(default_asset.into_inner()?, Some(import_data.into_inner()?));
        Ok(())
    }
}

#[derive(Hash, Serialize, Deserialize)]
pub struct GpuImageJobInput {
    pub asset_id: AssetId,
}
impl JobInput for GpuImageJobInput {}

#[derive(Serialize, Deserialize)]
pub struct GpuImageJobOutput {}
impl JobOutput for GpuImageJobOutput {}

#[derive(Default, TypeUuid)]
#[uuid = "5311c92e-470e-4fdc-88cd-3abaf1c28f39"]
pub struct GpuImageJobProcessor;

impl JobProcessor for GpuImageJobProcessor {
    type InputT = GpuImageJobInput;
    type OutputT = GpuImageJobOutput;

    fn version(&self) -> u32 {
        1
    }

    fn run<'a>(
        &self,
        context: &'a RunContext<'a, Self::InputT>,
    ) -> PipelineResult<GpuImageJobOutput> {
        //
        // Read asset properties
        //
        let asset_data = context
            .asset::<GpuImageAssetRecord>(context.input.asset_id)
            .unwrap();
        let basis_compression = asset_data.basis_compression().get().unwrap();
        let color_space = match asset_data.color_space().get().unwrap() {
            GpuImageColorSpaceEnum::Srgb => ImageAssetColorSpaceConfig::Srgb,
            GpuImageColorSpaceEnum::Linear => ImageAssetColorSpaceConfig::Linear,
        };
        let mip_generation = match asset_data.mip_generation().get().unwrap() {
            GpuImageMipGenerationEnum::NoMips => ImageAssetMipGeneration::NoMips,
            GpuImageMipGenerationEnum::Precomputed => ImageAssetMipGeneration::Precomupted,
            GpuImageMipGenerationEnum::Runtime => ImageAssetMipGeneration::Runtime,
        };

        let format_config = if basis_compression {
            let compression_type = match asset_data
                .basis_compression_settings()
                .compression_type()
                .get()
                .unwrap()
            {
                GpuImageBasisCompressionTypeEnum::Uastc => ImageAssetBasisCompressionType::Uastc,
                GpuImageBasisCompressionTypeEnum::Etc1S => ImageAssetBasisCompressionType::Etc1S,
            };
            let quality = asset_data
                .basis_compression_settings()
                .quality()
                .get()
                .unwrap();

            ImageAssetDataFormatConfig::BasisCompressed(ImageAssetBasisCompressionSettings {
                compression_type,
                quality,
            })
        } else {
            ImageAssetDataFormatConfig::Uncompressed
        };

        //
        // Read imported data
        //
        let import_data = context
            .imported_data::<GpuImageImportedDataRecord>(context.input.asset_id)
            .unwrap();

        let image_bytes = import_data.image_bytes().get().unwrap().clone();
        let width = import_data.width().get().unwrap();
        let height = import_data.height().get().unwrap();

        //
        // Create the processed data
        //
        let processed_data = ImageAssetData::from_raw_rgba32(
            width,
            height,
            color_space,
            format_config,
            mip_generation,
            RafxResourceType::TEXTURE,
            &image_bytes,
        )
        .unwrap();

        //
        // Serialize and return
        //
        context.produce_default_artifact(context.input.asset_id, processed_data)?;

        Ok(GpuImageJobOutput {})
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "7fe7e10b-6b99-4acc-8bf9-09cc17fedcdf"]
pub struct GpuImageBuilder {}

impl Builder for GpuImageBuilder {
    fn asset_type(&self) -> &'static str {
        GpuImageAssetAccessor::schema_name()
    }

    fn start_jobs(
        &self,
        context: BuilderContext,
    ) -> PipelineResult<()> {
        //Future: Might produce jobs per-platform
        context.enqueue_job::<GpuImageJobProcessor>(
            context.data_set,
            context.schema_set,
            context.job_api,
            GpuImageJobInput {
                asset_id: context.asset_id,
            },
        )?;
        Ok(())
    }
}
