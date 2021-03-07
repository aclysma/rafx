pub mod asset_loader;
pub use asset_loader::*;

pub mod asset_resource;
pub use asset_resource::*;

pub mod asset_storage;
pub use asset_storage::*;

pub fn default_daemon() -> distill::daemon::AssetDaemon {
    use crate::assets::*;

    distill::daemon::AssetDaemon::default()
        .with_importer("sampler", SamplerImporter)
        .with_importer("material", MaterialImporter)
        .with_importer("materialinstance", MaterialInstanceImporter)
        .with_importer("compute", ComputePipelineImporter)
        .with_importer("cookedshaderpackage", ShaderImporterCooked)
        .with_importer("png", ImageImporter)
        .with_importer("jpg", ImageImporter)
        .with_importer("jpeg", ImageImporter)
        .with_importer("tga", ImageImporter)
        .with_importer("bmp", ImageImporter)
        .with_importer("basis", BasisImageImporter)
}
