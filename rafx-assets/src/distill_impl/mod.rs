pub mod asset_loader;
pub use asset_loader::*;

pub mod asset_resource;
pub use asset_resource::*;

pub mod asset_storage;
pub use asset_storage::*;

pub fn default_daemon() -> distill::daemon::AssetDaemon {
    use crate::assets::*;

    #[allow(unused_mut)]
    let mut daemon = distill::daemon::AssetDaemon::default()
        .with_importer("sampler", SamplerImporter)
        .with_importer("material", MaterialImporter)
        .with_importer("materialinstance", MaterialInstanceImporter)
        .with_importer("compute", ComputePipelineImporter)
        .with_importer("cookedshaderpackage", ShaderImporterCooked)
        .with_importer("png", ImageImporter(image::ImageFormat::Png))
        .with_importer("jpg", ImageImporter(image::ImageFormat::Jpeg))
        .with_importer("jpeg", ImageImporter(image::ImageFormat::Jpeg))
        .with_importer("tga", ImageImporter(image::ImageFormat::Tga))
        .with_importer("tif", ImageImporter(image::ImageFormat::Tiff))
        .with_importer("tiff", ImageImporter(image::ImageFormat::Tiff))
        .with_importer("bmp", ImageImporter(image::ImageFormat::Bmp));

    #[cfg(feature = "basis-universal")]
    {
        daemon = daemon.with_importer("basis", BasisImageImporter);
    }

    daemon
}
