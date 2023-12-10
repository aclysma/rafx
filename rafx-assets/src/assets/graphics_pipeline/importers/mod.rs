use hydrate_pipeline::{
    AssetPlugin, BuilderRegistryBuilder, ImporterRegistryBuilder, JobProcessorRegistryBuilder,
    SchemaLinker,
};

pub mod material_importer;
use crate::assets::graphics_pipeline::material_instance_importer::{
    MaterialInstanceBuilder, MaterialInstanceImporter, MaterialInstanceJobProcessor,
};
use material_importer::{MaterialBuilder, MaterialImporter, MaterialJobProcessor};

pub mod material_instance_importer;

pub struct MaterialAssetPlugin;

impl AssetPlugin for MaterialAssetPlugin {
    fn setup(
        importer_registry: &mut ImporterRegistryBuilder,
        builder_registry: &mut BuilderRegistryBuilder,
        job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<MaterialImporter>();
        builder_registry.register_handler::<MaterialBuilder>();
        job_processor_registry.register_job_processor::<MaterialJobProcessor>();

        importer_registry.register_handler::<MaterialInstanceImporter>();
        builder_registry.register_handler::<MaterialInstanceBuilder>();
        job_processor_registry.register_job_processor::<MaterialInstanceJobProcessor>();
    }
}
