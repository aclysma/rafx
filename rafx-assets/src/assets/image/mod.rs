pub mod assets;

pub use assets::*;
use hydrate_data::SchemaLinker;
use hydrate_model::{BuilderRegistryBuilder, ImporterRegistryBuilder, JobProcessorRegistryBuilder};
use std::sync::Arc;

//mod importer;
//pub use importer::*;

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
#[cfg(feature = "ddsfile")]
pub use importer_dds::*;

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

        importer_registry.register_handler::<GpuCompressedImageImporterDds>(schema_linker);
        importer_registry.register_handler::<GpuCompressedImageImporterBasis>(schema_linker);

        builder_registry.register_handler::<GpuCompressedImageBuilder>(schema_linker);
        job_processor_registry.register_job_processor::<GpuCompressedImageJobProcessor>();
    }
}
