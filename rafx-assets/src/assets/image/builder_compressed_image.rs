use crate::assets::image::{
    ImageAssetData, ImageAssetDataLayer, ImageAssetDataMipLevel, ImageAssetDataPayload,
};
use crate::schema::{
    GpuCompressedImageAssetRecord, GpuCompressedImageImportedDataRecord,
    GpuImageAssetDataFormatEnum,
};
use crate::{
    ImageAssetDataFormat, ImageAssetDataPayloadSingleBuffer, ImageAssetDataPayloadSubresources,
};
use hydrate_base::hashing::HashMap;
use hydrate_base::AssetId;
use hydrate_data::{DataContainer, DataSet, Field, PropertyPath, Record, SchemaSet, SingleObject};
use hydrate_pipeline::{
    job_system, Builder, JobApi, JobEnumeratedDependencies, JobInput, JobOutput, JobProcessor,
};
use rafx_api::RafxResourceType;
use serde::{Deserialize, Serialize};
use type_uuid::*;

#[derive(Hash, Serialize, Deserialize)]
pub struct GpuCompressedImageJobInput {
    pub asset_id: AssetId,
}
impl JobInput for GpuCompressedImageJobInput {}

#[derive(Serialize, Deserialize)]
pub struct GpuCompressedImageJobOutput {}
impl JobOutput for GpuCompressedImageJobOutput {}

#[derive(Default, TypeUuid)]
#[uuid = "9377c690-a9d4-4744-bc43-bdc4b0c72c48"]
pub struct GpuCompressedImageJobProcessor;

impl JobProcessor for GpuCompressedImageJobProcessor {
    type InputT = GpuCompressedImageJobInput;
    type OutputT = GpuCompressedImageJobOutput;

    fn version(&self) -> u32 {
        1
    }

    fn enumerate_dependencies(
        &self,
        input: &GpuCompressedImageJobInput,
        _data_set: &DataSet,
        _schema_set: &SchemaSet,
    ) -> JobEnumeratedDependencies {
        // No dependencies
        JobEnumeratedDependencies {
            import_data: vec![input.asset_id],
            upstream_jobs: Vec::default(),
        }
    }

    fn run(
        &self,
        input: &GpuCompressedImageJobInput,
        _data_set: &DataSet,
        schema_set: &SchemaSet,
        dependency_data: &HashMap<AssetId, SingleObject>,
        job_api: &dyn JobApi,
    ) -> GpuCompressedImageJobOutput {
        //
        // Read asset properties
        //

        //
        // Read imported data
        //
        let imported_data = &dependency_data[&input.asset_id];
        let data_container = DataContainer::from_single_object(&imported_data, schema_set);
        let x = GpuCompressedImageImportedDataRecord::new(PropertyPath::default());

        let width = x.width().get(&data_container).unwrap();
        let height = x.height().get(&data_container).unwrap();
        let format = match x.format().get(&data_container).unwrap() {
            GpuImageAssetDataFormatEnum::RGBA32_Linear => ImageAssetDataFormat::RGBA32_Linear,
            GpuImageAssetDataFormatEnum::RGBA32_Srgb => ImageAssetDataFormat::RGBA32_Srgb,
            GpuImageAssetDataFormatEnum::Basis_Linear => ImageAssetDataFormat::Basis_Linear,
            GpuImageAssetDataFormatEnum::Basis_Srgb => ImageAssetDataFormat::Basis_Srgb,
            GpuImageAssetDataFormatEnum::BC1_UNorm_Linear => ImageAssetDataFormat::BC1_UNorm_Linear,
            GpuImageAssetDataFormatEnum::BC1_UNorm_Srgb => ImageAssetDataFormat::BC1_UNorm_Srgb,
            GpuImageAssetDataFormatEnum::BC2_UNorm_Linear => ImageAssetDataFormat::BC2_UNorm_Linear,
            GpuImageAssetDataFormatEnum::BC2_UNorm_Srgb => ImageAssetDataFormat::BC2_UNorm_Srgb,
            GpuImageAssetDataFormatEnum::BC3_UNorm_Linear => ImageAssetDataFormat::BC3_UNorm_Linear,
            GpuImageAssetDataFormatEnum::BC3_UNorm_Srgb => ImageAssetDataFormat::BC3_UNorm_Srgb,
            GpuImageAssetDataFormatEnum::BC4_UNorm => ImageAssetDataFormat::BC4_UNorm,
            GpuImageAssetDataFormatEnum::BC4_SNorm => ImageAssetDataFormat::BC4_SNorm,
            GpuImageAssetDataFormatEnum::BC5_UNorm => ImageAssetDataFormat::BC5_UNorm,
            GpuImageAssetDataFormatEnum::BC5_SNorm => ImageAssetDataFormat::BC5_SNorm,
            GpuImageAssetDataFormatEnum::BC6H_UFloat => ImageAssetDataFormat::BC6H_UFloat,
            GpuImageAssetDataFormatEnum::BC6H_SFloat => ImageAssetDataFormat::BC6H_SFloat,
            GpuImageAssetDataFormatEnum::BC7_Unorm_Linear => ImageAssetDataFormat::BC7_Unorm_Linear,
            GpuImageAssetDataFormatEnum::BC7_Unorm_Srgb => ImageAssetDataFormat::BC7_Unorm_Srgb,
        };
        let resource_type = if x.is_cube_texture().get(&data_container).unwrap() {
            RafxResourceType::TEXTURE_CUBE
        } else {
            RafxResourceType::TEXTURE
        };

        let layer_entries = x.data_layers().resolve_entries(&data_container).unwrap();
        let payload = if layer_entries.is_empty() {
            ImageAssetDataPayload::SingleBuffer(ImageAssetDataPayloadSingleBuffer {
                buffer: x.data_single_buffer().get(&data_container).unwrap().clone(),
            })
        } else {
            let mut layers = Vec::default();
            for &layer_entry in layer_entries.into_iter() {
                let layer = x.data_layers().entry(layer_entry);
                let mip_level_entries =
                    layer.mip_levels().resolve_entries(&data_container).unwrap();
                let mut mip_levels = Vec::default();
                for &mip_level_entry in mip_level_entries.into_iter() {
                    let mip_level = layer.mip_levels().entry(mip_level_entry);
                    mip_levels.push(ImageAssetDataMipLevel {
                        width: mip_level.width().get(&data_container).unwrap(),
                        height: mip_level.height().get(&data_container).unwrap(),
                        bytes: mip_level.bytes().get(&data_container).unwrap().clone(),
                    });
                }

                layers.push(ImageAssetDataLayer { mip_levels });
            }

            ImageAssetDataPayload::Subresources(ImageAssetDataPayloadSubresources { layers })
        };

        //
        // Create the processed data
        //
        let processed_data = ImageAssetData {
            width,
            height,
            format,
            resource_type,
            generate_mips_at_runtime: false,
            data: payload,
        };

        //
        // Serialize and return
        //
        job_system::produce_asset(job_api, input.asset_id, processed_data);

        GpuCompressedImageJobOutput {}
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "69ec69e0-feb2-4fca-a2a3-8fb963a26dfc"]
pub struct GpuCompressedImageBuilder {}

impl Builder for GpuCompressedImageBuilder {
    fn asset_type(&self) -> &'static str {
        GpuCompressedImageAssetRecord::schema_name()
    }

    fn start_jobs(
        &self,
        asset_id: AssetId,
        data_set: &DataSet,
        schema_set: &SchemaSet,
        job_api: &dyn JobApi,
    ) {
        //Future: Might produce jobs per-platform
        job_system::enqueue_job::<GpuCompressedImageJobProcessor>(
            data_set,
            schema_set,
            job_api,
            GpuCompressedImageJobInput { asset_id },
        );
    }
}
