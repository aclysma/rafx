pub mod asset_loader;

pub use asset_loader::*;
use std::sync::Arc;

pub mod asset_resource;
pub use asset_resource::*;

pub mod asset_storage;
pub use asset_storage::*;

use hydrate_model::{AssetPluginRegistrationHelper, SchemaLinker};

pub fn register_default_hydrate_plugins(
    mut registration_helper: AssetPluginRegistrationHelper,
    schema_linker: &mut SchemaLinker,
) -> AssetPluginRegistrationHelper {
    use crate::assets::*;

    registration_helper = registration_helper
        .register_plugin::<GpuImageAssetPlugin>(schema_linker)
        .register_plugin::<ShaderPackageAssetPlugin>(schema_linker)
        .register_plugin::<MaterialAssetPlugin>(schema_linker)
        .register_plugin::<ComputePipelineAssetPlugin>(schema_linker);

    registration_helper
    //TODO: Material instance
    //TODO: Sampler

    /*
    #[allow(unused_mut)]
    let mut daemon = distill::daemon::AssetDaemon::default()
        .with_importer(&["material"], MaterialImporter)
        .with_importer(&["materialinstance"], MaterialInstanceImporter)
        .with_importer(&["compute"], ComputePipelineImporter)
        .with_importer(&["cookedshaderpackage"], ShaderImporterCooked)
        .with_importer(
            &["png"],
            ImageImporter(ImageFileFormat::Png, image_importer_config.clone()),
        )
        .with_importer(
            &["jpg", "jpeg"],
            ImageImporter(ImageFileFormat::Jpeg, image_importer_config.clone()),
        )
        .with_importer(
            &["tga"],
            ImageImporter(ImageFileFormat::Tga, image_importer_config.clone()),
        )
        .with_importer(
            &["tif", "tiff"],
            ImageImporter(ImageFileFormat::Tiff, image_importer_config.clone()),
        )
        .with_importer(
            &["bmp"],
            ImageImporter(ImageFileFormat::Bmp, image_importer_config.clone()),
        );

    #[cfg(feature = "basis-universal")]
    {
        daemon = daemon.with_importer(&["basis"], BasisImageImporter);
    }

    #[cfg(feature = "ddsfile")]
    {
        daemon = daemon.with_importer(&["dds"], DdsImageImporter);
    }
     */
}
