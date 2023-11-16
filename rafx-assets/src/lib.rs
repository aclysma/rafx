//! Provides integration with the [`distill`](https://github.com/amethyst/distill) asset
//! pipeline

pub mod assets;

pub use assets::*;

/// Contains some distill-related helpers. They are optional and end-users can provide their own.
mod push_buffer;
pub use push_buffer::PushBuffer;
pub use push_buffer::PushBufferResult;
pub use push_buffer::PushBufferSizeCalculator;

// mod resource_loader;
// pub use resource_loader::ResourceLoader;

pub mod schema;

mod hydrate_impl;
pub use hydrate_impl::AssetResource;
pub use hydrate_impl::RafxResourceAssetLoader;

pub use hydrate_base::Handle;

mod resource_loader;

use hydrate_pipeline::{AssetPluginRegistrationHelper, SchemaLinker};
use std::path::PathBuf;

pub fn schema_def_path() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/schema"))
}

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
