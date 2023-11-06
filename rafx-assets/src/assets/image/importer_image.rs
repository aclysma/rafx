use crate::assets::image::{
    ImageAssetColorSpaceConfig, ImageAssetData, ImageAssetDataFormatConfig,
};
use crate::schema::{
    GpuCompressedImageAssetRecord, GpuImageAssetRecord, GpuImageBasisCompressionTypeEnum,
    GpuImageColorSpaceEnum, GpuImageImportedDataRecord, GpuImageMipGenerationEnum,
};
use crate::{
    ImageAssetBasisCompressionSettings, ImageAssetBasisCompressionType, ImageAssetDataFormat,
    ImageAssetMipGeneration,
};
use hydrate_base::hashing::HashMap;
use hydrate_base::ObjectId;
use hydrate_data::{
    DataContainer, DataContainerMut, DataSet, DataSetResult, EnumField, Field, PropertyPath,
    Record, SchemaLinker, SchemaSet, SingleObject,
};
use hydrate_model::{
    job_system, Builder, BuilderRegistryBuilder, ImportableObject, ImportedImportable,
    ImporterRegistry, ImporterRegistryBuilder, JobApi, JobEnumeratedDependencies, JobInput,
    JobOutput, JobProcessor, JobProcessorRegistryBuilder, ScannedImportable,
};
use image::GenericImageView;
use rafx_api::RafxResourceType;
use rafx_framework::upload::GpuImageDataColorSpace;
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
//
// #[derive(TypeUuid, Serialize, Deserialize, Default)]
// #[uuid = "23f90369-6916-4548-81d0-a76e0b162df2"]
// pub struct ImageImporterState(Option<AssetUuid>);
//
// #[derive(TypeUuid)]
// #[uuid = "4ae5ddc5-6805-4cf5-aa14-d44c6e0b8251"]
// pub struct ImageImporter(pub ImageFileFormat, pub Arc<ImageImporterConfig>);
// impl Importer for ImageImporter {
//     fn version_static() -> u32
//     where
//         Self: Sized,
//     {
//         7
//     }
//
//     fn version(&self) -> u32 {
//         Self::version_static()
//     }
//
//     type Options = ImageImporterOptions;
//
//     type State = ImageImporterState;
//
//     fn default_options(
//         &self,
//         import_source: ImportSource,
//     ) -> Option<Self::Options> {
//         match import_source {
//             ImportSource::File(path) => {
//                 for rule in &self.1.rules {
//                     if let Some(options) = rule.try_apply(path) {
//                         log::trace!("FOUND RULE FOR {:?}", path);
//                         return Some(ImageImporterOptions {
//                             mip_generation: options.mip_generation,
//                             data_format: options.data_format,
//                             color_space: options.color_space,
//                         });
//                     }
//                 }
//             }
//         }
//
//         return Some(ImageImporterOptions {
//             mip_generation: self.1.default.mip_generation,
//             data_format: self.1.default.data_format,
//             color_space: self.1.default.color_space,
//         });
//     }
//
//     /// Reads the given bytes and produces assets.
//     #[profiling::function]
//     fn import(
//         &self,
//         _op: &mut ImportOp,
//         source: &mut dyn Read,
//         options: &Self::Options,
//         state: &mut Self::State,
//     ) -> distill::importer::Result<ImporterValue> {
//         let id = state
//             .0
//             .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
//         *state = ImageImporterState(Some(id));
//         let mut bytes = Vec::new();
//         source.read_to_end(&mut bytes)?;
//
//         use image::EncodableLayout;
//
//         log::trace!("import with options {:?}", options);
//
//         let decoded_image = image::load_from_memory_with_format(&bytes, self.0.into())
//             .map_err(|e| Error::Boxed(Box::new(e)))?;
//         let (width, height) = decoded_image.dimensions();
//         let asset_data = ImageAssetData::from_raw_rgba32(
//             width,
//             height,
//             options.color_space,
//             options.data_format,
//             options.mip_generation,
//             RafxResourceType::TEXTURE,
//             decoded_image.into_rgba8().as_bytes(),
//         )
//         .unwrap();
//
//         Ok(ImporterValue {
//             assets: vec![ImportedAsset {
//                 id,
//                 search_tags: vec![],
//                 build_deps: vec![],
//                 load_deps: vec![],
//                 build_pipeline: None,
//                 asset_data: Box::new(asset_data),
//             }],
//         })
//     }
// }

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
        default_asset_data_container: &mut DataContainerMut,
        asset_record: &GpuImageAssetRecord,
    ) {
        match default_settings.data_format {
            ImageAssetDataFormatConfig::Uncompressed => {
                asset_record
                    .basis_compression()
                    .set(default_asset_data_container, false)
                    .unwrap();
            }
            ImageAssetDataFormatConfig::BasisCompressed(settings) => {
                asset_record
                    .basis_compression()
                    .set(default_asset_data_container, true)
                    .unwrap();

                asset_record
                    .basis_compression_settings()
                    .compression_type()
                    .set(
                        default_asset_data_container,
                        match settings.compression_type {
                            ImageAssetBasisCompressionType::Etc1S => {
                                GpuImageBasisCompressionTypeEnum::Etc1S
                            }
                            ImageAssetBasisCompressionType::Uastc => {
                                GpuImageBasisCompressionTypeEnum::Uastc
                            }
                        },
                    )
                    .unwrap();
                asset_record
                    .basis_compression_settings()
                    .quality()
                    .set(default_asset_data_container, settings.quality)
                    .unwrap();
            }
        }
        asset_record
            .color_space()
            .set(
                default_asset_data_container,
                match default_settings.color_space {
                    ImageAssetColorSpaceConfig::Srgb => GpuImageColorSpaceEnum::Srgb,
                    ImageAssetColorSpaceConfig::Linear => GpuImageColorSpaceEnum::Linear,
                },
            )
            .unwrap();
        asset_record
            .mip_generation()
            .set(
                default_asset_data_container,
                match default_settings.mip_generation {
                    ImageAssetMipGeneration::NoMips => GpuImageMipGenerationEnum::NoMips,
                    ImageAssetMipGeneration::Precomupted => GpuImageMipGenerationEnum::Precomputed,
                    ImageAssetMipGeneration::Runtime => GpuImageMipGenerationEnum::Runtime,
                },
            )
            .unwrap();
    }
}

impl hydrate_model::Importer for GpuImageImporterSimple {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["png", "jpg", "jpeg", "tga", "tif", "tiff", "bmp"]
    }

    fn scan_file(
        &self,
        path: &Path,
        schema_set: &SchemaSet,
        importer_registry: &ImporterRegistry,
    ) -> Vec<ScannedImportable> {
        let asset_type = schema_set
            .find_named_type(GpuImageAssetRecord::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();
        vec![ScannedImportable {
            name: None,
            asset_type,
            file_references: Default::default(),
        }]
    }

    fn import_file(
        &self,
        path: &Path,
        importable_objects: &HashMap<Option<String>, ImportableObject>,
        schema_set: &SchemaSet,
    ) -> HashMap<Option<String>, ImportedImportable> {
        let decoded_image = ::image::open(path).unwrap();
        let (width, height) = decoded_image.dimensions();
        let image_bytes = decoded_image.into_rgba8().to_vec();

        //
        // Create import data
        //
        let import_data = {
            let mut import_object =
                GpuImageImportedDataRecord::new_single_object(schema_set).unwrap();
            let mut import_data_container =
                DataContainerMut::new_single_object(&mut import_object, schema_set);
            let x = GpuImageImportedDataRecord::default();
            x.image_bytes()
                .set(&mut import_data_container, image_bytes)
                .unwrap();
            x.width().set(&mut import_data_container, width).unwrap();
            x.height().set(&mut import_data_container, height).unwrap();
            import_object
        };

        //
        // Create the default asset
        //
        let default_asset = {
            let default_settings = self.default_settings(path);

            let mut default_asset_object =
                GpuImageAssetRecord::new_single_object(schema_set).unwrap();
            let mut default_asset_data_container =
                DataContainerMut::new_single_object(&mut default_asset_object, schema_set);
            let x = GpuImageAssetRecord::default();

            GpuImageImporterSimple::set_default_asset_properties(
                &default_settings,
                &mut default_asset_data_container,
                &x,
            );

            // match default_settings.data_format {
            //     ImageAssetDataFormatConfig::Uncompressed => {
            //         x.basis_compression()
            //             .set(&mut default_asset_data_container, false)
            //             .unwrap();
            //     }
            //     ImageAssetDataFormatConfig::BasisCompressed(settings) => {
            //         x.basis_compression()
            //             .set(&mut default_asset_data_container, true)
            //             .unwrap();
            //
            //         x.basis_compression_settings()
            //             .compression_type()
            //             .set(
            //                 &mut default_asset_data_container,
            //                 match settings.compression_type {
            //                     ImageAssetBasisCompressionType::Etc1S => {
            //                         GpuImageBasisCompressionTypeEnum::Etc1S
            //                     }
            //                     ImageAssetBasisCompressionType::Uastc => {
            //                         GpuImageBasisCompressionTypeEnum::Uastc
            //                     }
            //                 },
            //             )
            //             .unwrap();
            //         x.basis_compression_settings()
            //             .quality()
            //             .set(&mut default_asset_data_container, settings.quality)
            //             .unwrap();
            //     }
            // }
            // x.color_space()
            //     .set(
            //         &mut default_asset_data_container,
            //         match default_settings.color_space {
            //             ImageAssetColorSpaceConfig::Srgb => GpuImageColorSpaceEnum::Srgb,
            //             ImageAssetColorSpaceConfig::Linear => GpuImageColorSpaceEnum::Linear,
            //         },
            //     )
            //     .unwrap();
            // x.mip_generation()
            //     .set(
            //         &mut default_asset_data_container,
            //         match default_settings.mip_generation {
            //             ImageAssetMipGeneration::NoMips => GpuImageMipGenerationEnum::NoMips,
            //             ImageAssetMipGeneration::Precomupted => {
            //                 GpuImageMipGenerationEnum::Precomputed
            //             }
            //             ImageAssetMipGeneration::Runtime => GpuImageMipGenerationEnum::Runtime,
            //         },
            //     )
            //     .unwrap();

            default_asset_object
        };

        //
        // Return the created objects
        //
        let mut imported_objects = HashMap::default();
        imported_objects.insert(
            None,
            ImportedImportable {
                file_references: Default::default(),
                import_data: Some(import_data),
                default_asset: Some(default_asset),
            },
        );
        imported_objects
    }
}

#[derive(Hash, Serialize, Deserialize)]
pub struct GpuImageJobInput {
    pub asset_id: ObjectId,
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

    fn enumerate_dependencies(
        &self,
        input: &GpuImageJobInput,
        data_set: &DataSet,
        schema_set: &SchemaSet,
    ) -> JobEnumeratedDependencies {
        // No dependencies
        JobEnumeratedDependencies {
            import_data: vec![input.asset_id],
            upstream_jobs: Vec::default(),
        }
    }

    fn run(
        &self,
        input: &GpuImageJobInput,
        data_set: &DataSet,
        schema_set: &SchemaSet,
        dependency_data: &HashMap<ObjectId, SingleObject>,
        job_api: &dyn JobApi,
    ) -> GpuImageJobOutput {
        //
        // Read asset properties
        //
        let data_container = DataContainer::new_dataset(data_set, schema_set, input.asset_id);
        let x = GpuImageAssetRecord::default();
        let basis_compression = x.basis_compression().get(&data_container).unwrap();
        let color_space = match x.color_space().get(&data_container).unwrap() {
            GpuImageColorSpaceEnum::Srgb => ImageAssetColorSpaceConfig::Srgb,
            GpuImageColorSpaceEnum::Linear => ImageAssetColorSpaceConfig::Linear,
        };
        let mip_generation = match x.mip_generation().get(&data_container).unwrap() {
            GpuImageMipGenerationEnum::NoMips => ImageAssetMipGeneration::NoMips,
            GpuImageMipGenerationEnum::Precomputed => ImageAssetMipGeneration::Precomupted,
            GpuImageMipGenerationEnum::Runtime => ImageAssetMipGeneration::Runtime,
        };

        let format_config = if basis_compression {
            let compression_type = match x
                .basis_compression_settings()
                .compression_type()
                .get(&data_container)
                .unwrap()
            {
                GpuImageBasisCompressionTypeEnum::Uastc => ImageAssetBasisCompressionType::Uastc,
                GpuImageBasisCompressionTypeEnum::Etc1S => ImageAssetBasisCompressionType::Etc1S,
            };
            let quality = x
                .basis_compression_settings()
                .quality()
                .get(&data_container)
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
        let imported_data = &dependency_data[&input.asset_id];
        let data_container = DataContainer::new_single_object(&imported_data, schema_set);
        let x = GpuImageImportedDataRecord::new(PropertyPath::default());

        let image_bytes = x.image_bytes().get(&data_container).unwrap().clone();
        let width = x.width().get(&data_container).unwrap();
        let height = x.height().get(&data_container).unwrap();

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
        job_system::produce_asset(job_api, input.asset_id, processed_data);

        GpuImageJobOutput {}
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "7fe7e10b-6b99-4acc-8bf9-09cc17fedcdf"]
pub struct GpuImageBuilder {}

impl Builder for GpuImageBuilder {
    fn asset_type(&self) -> &'static str {
        GpuImageAssetRecord::schema_name()
    }

    fn start_jobs(
        &self,
        asset_id: ObjectId,
        data_set: &DataSet,
        schema_set: &SchemaSet,
        job_api: &dyn JobApi,
    ) {
        //Future: Might produce jobs per-platform
        job_system::enqueue_job::<GpuImageJobProcessor>(
            data_set,
            schema_set,
            job_api,
            GpuImageJobInput { asset_id },
        );
    }
}

pub struct GpuImageAssetPlugin;

impl hydrate_model::AssetPlugin for GpuImageAssetPlugin {
    fn setup(
        schema_linker: &mut SchemaLinker,
        importer_registry: &mut ImporterRegistryBuilder,
        builder_registry: &mut BuilderRegistryBuilder,
        job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        // This demonstrates using filenames to hint default settings for images on import for normal
        // maps and roughness/metalness maps by using filenames. Otherwise, the user has to remember to
        // edit the .meta file.
        let pbr_map_suffix = vec!["_pbr."];
        let normal_map_suffix = vec!["_n."];

        // Default config
        let mut image_importer_config = ImageImporterConfig::new(ImageImporterRuleOptions {
            mip_generation: ImageAssetMipGeneration::Runtime,
            color_space: ImageAssetColorSpaceConfig::Srgb,
            data_format: ImageAssetDataFormatConfig::Uncompressed,
        });

        for suffix in normal_map_suffix {
            // Override for normal maps
            image_importer_config.add_filename_contains_override(
                suffix,
                ImageImporterRuleOptions {
                    mip_generation: ImageAssetMipGeneration::Runtime,
                    color_space: ImageAssetColorSpaceConfig::Linear,
                    data_format: ImageAssetDataFormatConfig::Uncompressed,
                },
            );
        }

        // Override for PBR masks (ao, roughness, metalness)
        for suffix in pbr_map_suffix {
            image_importer_config.add_filename_contains_override(
                suffix,
                ImageImporterRuleOptions {
                    mip_generation: ImageAssetMipGeneration::Runtime,
                    color_space: ImageAssetColorSpaceConfig::Linear,
                    data_format: ImageAssetDataFormatConfig::Uncompressed,
                },
            );
        }

        let image_importer_config = Arc::new(image_importer_config);

        importer_registry.register_handler_instance::<GpuImageImporterSimple>(
            schema_linker,
            GpuImageImporterSimple {
                image_importer_config,
            },
        );
        builder_registry.register_handler::<GpuImageBuilder>(schema_linker);
        job_processor_registry.register_job_processor::<GpuImageJobProcessor>();
    }
}
