pub mod assets;

pub use assets::*;
use hydrate_data::SchemaLinker;
use hydrate_pipeline::{
    BuilderRegistryBuilder, ImporterRegistryBuilder, JobProcessorRegistryBuilder,
};
use std::sync::Arc;

mod asset_upload_queue;

mod asset_type_handler;
pub use asset_type_handler::*;

mod importer_image;
pub use importer_image::*;

#[cfg(feature = "basis-universal")]
mod importer_basis;
#[cfg(feature = "basis-universal")]
pub use importer_basis::*;

#[cfg(feature = "ddsfile")]
mod importer_dds;
use crate::assets::image::builder_compressed_image::{
    GpuCompressedImageBuilder, GpuCompressedImageJobProcessor,
};
#[cfg(feature = "ddsfile")]
pub use importer_dds::*;

mod builder_compressed_image;

pub struct GpuImageAssetPlugin;

impl hydrate_pipeline::AssetPlugin for GpuImageAssetPlugin {
    fn setup(
        _schema_linker: &mut SchemaLinker,
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
            GpuImageImporterSimple {
                image_importer_config,
            },
        );
        builder_registry.register_handler::<GpuImageBuilder>();
        job_processor_registry.register_job_processor::<GpuImageJobProcessor>();

        #[cfg(feature = "ddsfile")]
        importer_registry.register_handler::<GpuCompressedImageImporterDds>();
        #[cfg(feature = "basis-universal")]
        importer_registry.register_handler::<GpuCompressedImageImporterBasis>();

        builder_registry.register_handler::<GpuCompressedImageBuilder>();
        job_processor_registry.register_job_processor::<GpuCompressedImageJobProcessor>();
    }
}
