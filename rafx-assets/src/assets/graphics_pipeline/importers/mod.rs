use hydrate_pipeline::{AssetPlugin, AssetPluginSetupContext};

pub mod material_importer;
use crate::assets::graphics_pipeline::material_instance_importer::{
    MaterialInstanceBuilder, MaterialInstanceImporter, MaterialInstanceJobProcessor,
};
use material_importer::{MaterialBuilder, MaterialImporter, MaterialJobProcessor};

pub mod material_instance_importer;

pub struct MaterialAssetPlugin;

impl AssetPlugin for MaterialAssetPlugin {
    fn setup(context: AssetPluginSetupContext) {
        context
            .importer_registry
            .register_handler::<MaterialImporter>();
        context
            .builder_registry
            .register_handler::<MaterialBuilder>();
        context
            .job_processor_registry
            .register_job_processor::<MaterialJobProcessor>();

        context
            .importer_registry
            .register_handler::<MaterialInstanceImporter>();
        context
            .builder_registry
            .register_handler::<MaterialInstanceBuilder>();
        context
            .job_processor_registry
            .register_job_processor::<MaterialInstanceJobProcessor>();
    }
}
