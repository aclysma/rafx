pub mod asset_loader;

pub use asset_loader::*;
use std::sync::Arc;

pub mod asset_resource;
pub use asset_resource::*;

pub mod asset_storage;
pub use asset_storage::*;

pub fn default_daemon() -> distill::daemon::AssetDaemon {
    use crate::assets::*;

    // This demonstrates using filenames to hint default settings for images on import for normal
    // maps and roughness/metalness maps by using filenames. Otherwise, the user has to remember to
    // edit the .meta file.
    let pbr_map_suffix = vec!["_pbr."];
    let normal_map_suffix = vec!["_n."];

    // Default config
    let mut image_importer_config = ImageImporterConfig::new(ImageImporterRuleOptions {
        mip_generation: ImageAssetMipGeneration::Runtime,
        color_space: ImageAssetColorSpace::Srgb,
        data_format: ImageAssetDataFormatConfig::Uncompressed,
    });

    for suffix in normal_map_suffix {
        // Override for normal maps
        image_importer_config.add_filename_contains_override(
            suffix,
            ImageImporterRuleOptions {
                mip_generation: ImageAssetMipGeneration::Runtime,
                color_space: ImageAssetColorSpace::Linear,
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
                color_space: ImageAssetColorSpace::Linear,
                data_format: ImageAssetDataFormatConfig::Uncompressed,
            },
        );
    }

    let image_importer_config = Arc::new(image_importer_config);

    #[allow(unused_mut)]
    let mut daemon = distill::daemon::AssetDaemon::default()
        .with_importer("sampler", SamplerImporter)
        .with_importer("material", MaterialImporter)
        .with_importer("materialinstance", MaterialInstanceImporter)
        .with_importer("compute", ComputePipelineImporter)
        .with_importer("cookedshaderpackage", ShaderImporterCooked)
        .with_importer(
            "png",
            ImageImporter(ImageFileFormat::Png, image_importer_config.clone()),
        )
        .with_importer(
            "jpg",
            ImageImporter(ImageFileFormat::Jpeg, image_importer_config.clone()),
        )
        .with_importer(
            "jpeg",
            ImageImporter(ImageFileFormat::Jpeg, image_importer_config.clone()),
        )
        .with_importer(
            "tga",
            ImageImporter(ImageFileFormat::Tga, image_importer_config.clone()),
        )
        .with_importer(
            "tif",
            ImageImporter(ImageFileFormat::Tiff, image_importer_config.clone()),
        )
        .with_importer(
            "tiff",
            ImageImporter(ImageFileFormat::Tiff, image_importer_config.clone()),
        )
        .with_importer(
            "bmp",
            ImageImporter(ImageFileFormat::Bmp, image_importer_config.clone()),
        );

    #[cfg(feature = "basis-universal")]
    {
        daemon = daemon.with_importer("basis", BasisImageImporter);
    }

    daemon
}
