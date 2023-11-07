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
use ddsfile::DxgiFormat;
use hydrate_base::hashing::HashMap;
use hydrate_base::ObjectId;
use hydrate_data::{
    DataContainer, DataContainerMut, DataSet, Field, PropertyPath, Record, SchemaSet, SingleObject,
};
use hydrate_model::{
    job_system, Builder, ImportableObject, ImportedImportable, ImporterRegistry, JobApi,
    JobEnumeratedDependencies, JobInput, JobOutput, JobProcessor, ScannedImportable,
};
use rafx_api::RafxResourceType;
use serde::{Deserialize, Serialize};
use std::path::Path;
use type_uuid::*;
//
// #[derive(TypeUuid, Serialize, Deserialize, Default)]
// #[uuid = "5eaeedae-8319-48f3-a50b-039f0613ec61"]
// pub struct DdsImageImporterState(Option<AssetUuid>);
//
// #[derive(TypeUuid)]
// #[uuid = "c1cedd9b-5f28-42e0-afc7-77891d7cadb4"]
// pub struct DdsImageImporter;
//
// impl Importer for DdsImageImporter {
//     fn version_static() -> u32
//     where
//         Self: Sized,
//     {
//         1
//     }
//
//     fn version(&self) -> u32 {
//         Self::version_static()
//     }
//
//     type Options = ();
//
//     type State = DdsImageImporterState;
//
//     /// Reads the given bytes and produces assets.
//     #[profiling::function]
//     fn import(
//         &self,
//         _op: &mut ImportOp,
//         source: &mut dyn Read,
//         _options: &Self::Options,
//         state: &mut Self::State,
//     ) -> distill::importer::Result<ImporterValue> {
//         let id = state
//             .0
//             .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
//         *state = DdsImageImporterState(Some(id));
//         let mut bytes = Vec::new();
//         source.read_to_end(&mut bytes)?;
//
//         let dds = ddsfile::Dds::read(&mut &bytes[..]).map_err(|e| Error::Boxed(Box::new(e)))?;
//
//         let format = if let Some(dxgi_format) = dds.get_dxgi_format() {
//             match dxgi_format {
//                 //DxgiFormat::BC1_Typeless => {}
//                 DxgiFormat::BC1_UNorm => ImageAssetDataFormat::BC1_UNorm_Linear,
//                 DxgiFormat::BC1_UNorm_sRGB => ImageAssetDataFormat::BC1_UNorm_Srgb,
//                 //DxgiFormat::BC2_Typeless => {}
//                 DxgiFormat::BC2_UNorm => ImageAssetDataFormat::BC2_UNorm_Linear,
//                 DxgiFormat::BC2_UNorm_sRGB => ImageAssetDataFormat::BC2_UNorm_Srgb,
//                 //DxgiFormat::BC3_Typeless => {}
//                 DxgiFormat::BC3_UNorm => ImageAssetDataFormat::BC3_UNorm_Linear,
//                 DxgiFormat::BC3_UNorm_sRGB => ImageAssetDataFormat::BC3_UNorm_Srgb,
//                 //DxgiFormat::BC4_Typeless => {}
//                 DxgiFormat::BC4_UNorm => ImageAssetDataFormat::BC4_UNorm,
//                 DxgiFormat::BC4_SNorm => ImageAssetDataFormat::BC4_SNorm,
//                 //DxgiFormat::BC5_Typeless => {}
//                 DxgiFormat::BC5_UNorm => ImageAssetDataFormat::BC5_UNorm,
//                 DxgiFormat::BC5_SNorm => ImageAssetDataFormat::BC5_SNorm,
//                 //DxgiFormat::BC6H_Typeless => {}
//                 DxgiFormat::BC6H_UF16 => ImageAssetDataFormat::BC6H_UFloat,
//                 DxgiFormat::BC6H_SF16 => ImageAssetDataFormat::BC6H_SFloat,
//                 //DxgiFormat::BC7_Typeless => {}
//                 DxgiFormat::BC7_UNorm => ImageAssetDataFormat::BC7_Unorm_Linear,
//                 DxgiFormat::BC7_UNorm_sRGB => ImageAssetDataFormat::BC7_Unorm_Srgb,
//                 _ => unimplemented!(),
//             }
//         } else {
//             unimplemented!();
//         };
//
//         let width = dds.get_width();
//         let height = dds.get_height();
//         let array_layer_count = dds.get_num_array_layers();
//         let mip_level_count = dds.get_num_mipmap_levels();
//
//         if dds.get_depth() != 1 {
//             unimplemented!("DDS importer only supports image depth = 1");
//         }
//
//         log::trace!(
//             "w: {} h: {} layers: {} mips: {} format: {:?} dxgi_format: {:?} d3d_format: {:?}",
//             width,
//             height,
//             array_layer_count,
//             mip_level_count,
//             format,
//             dds.get_dxgi_format(),
//             dds.get_d3d_format()
//         );
//         //println!("Import DDS texture: {:?}", dds);
//
//         let mut layers_asset_data = Vec::with_capacity(array_layer_count as usize);
//         for layer_index in 0..array_layer_count {
//             let layer = dds.get_data(0).map_err(|e| Error::Boxed(Box::new(e)))?;
//
//             let mut current_mipmap_size_bytes = dds.get_main_texture_size().unwrap() as usize;
//             let min_mipmap_size_bytes = dds.get_min_mipmap_size_in_bytes() as usize;
//             let mut offset_bytes = 0_usize;
//
//             let mut mip_width = width;
//             let mut mip_height = height;
//
//             let mut mip_levels_asset_data = Vec::with_capacity(mip_level_count as usize);
//             for mip_index in 0..mip_level_count {
//                 let mip_data: Vec<u8> = layer
//                     [offset_bytes..(offset_bytes + current_mipmap_size_bytes)]
//                     .iter()
//                     .copied()
//                     .collect();
//                 log::trace!(
//                     "Gathered mip data {} {} {}",
//                     layer_index,
//                     mip_index,
//                     mip_data.len()
//                 );
//
//                 mip_levels_asset_data.push(ImageAssetDataMipLevel {
//                     width: mip_width,
//                     height: mip_height,
//                     bytes: mip_data,
//                 });
//
//                 offset_bytes += current_mipmap_size_bytes;
//                 current_mipmap_size_bytes /= 4;
//                 if current_mipmap_size_bytes < min_mipmap_size_bytes {
//                     current_mipmap_size_bytes = min_mipmap_size_bytes;
//                 }
//
//                 mip_width /= 2;
//                 mip_height /= 2;
//             }
//
//             layers_asset_data.push(ImageAssetDataLayer {
//                 mip_levels: mip_levels_asset_data,
//             });
//         }
//
//         let asset_data = ImageAssetData {
//             width,
//             height,
//             format,
//             generate_mips_at_runtime: false,
//             resource_type: RafxResourceType::TEXTURE,
//             data: ImageAssetDataPayload::Subresources(ImageAssetDataPayloadSubresources {
//                 layers: layers_asset_data,
//             }),
//         };
//
//         Ok(ImporterValue {
//             assets: vec![ImportedAsset {
//                 id,
//                 search_tags: vec![],
//                 build_deps: vec![],
//                 load_deps: vec![],
//                 build_pipeline: None,
//                 asset_data: Box::new(asset_data),
//             }],
//         })
//     }
// }

#[derive(TypeUuid, Default)]
#[uuid = "a66a5767-0a03-4c3e-ac06-ce02c1a0a561"]
pub struct GpuCompressedImageImporterDds;

impl hydrate_model::Importer for GpuCompressedImageImporterDds {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["dds"]
    }

    fn scan_file(
        &self,
        _path: &Path,
        schema_set: &SchemaSet,
        _importer_registry: &ImporterRegistry,
    ) -> Vec<ScannedImportable> {
        let asset_type = schema_set
            .find_named_type(GpuCompressedImageAssetRecord::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();
        vec![ScannedImportable {
            name: None,
            asset_type,
            file_references: Default::default(),
        }]
    }

    fn import_file(
        &self,
        path: &Path,
        importable_objects: &HashMap<Option<String>, ImportableObject>,
        schema_set: &SchemaSet,
    ) -> HashMap<Option<String>, ImportedImportable> {
        let dds_bytes = std::fs::read(path).unwrap();
        let dds = ddsfile::Dds::read(&mut &dds_bytes[..]).unwrap();

        let format = if let Some(dxgi_format) = dds.get_dxgi_format() {
            match dxgi_format {
                //DxgiFormat::BC1_Typeless => {}
                DxgiFormat::BC1_UNorm => ImageAssetDataFormat::BC1_UNorm_Linear,
                DxgiFormat::BC1_UNorm_sRGB => ImageAssetDataFormat::BC1_UNorm_Srgb,
                //DxgiFormat::BC2_Typeless => {}
                DxgiFormat::BC2_UNorm => ImageAssetDataFormat::BC2_UNorm_Linear,
                DxgiFormat::BC2_UNorm_sRGB => ImageAssetDataFormat::BC2_UNorm_Srgb,
                //DxgiFormat::BC3_Typeless => {}
                DxgiFormat::BC3_UNorm => ImageAssetDataFormat::BC3_UNorm_Linear,
                DxgiFormat::BC3_UNorm_sRGB => ImageAssetDataFormat::BC3_UNorm_Srgb,
                //DxgiFormat::BC4_Typeless => {}
                DxgiFormat::BC4_UNorm => ImageAssetDataFormat::BC4_UNorm,
                DxgiFormat::BC4_SNorm => ImageAssetDataFormat::BC4_SNorm,
                //DxgiFormat::BC5_Typeless => {}
                DxgiFormat::BC5_UNorm => ImageAssetDataFormat::BC5_UNorm,
                DxgiFormat::BC5_SNorm => ImageAssetDataFormat::BC5_SNorm,
                //DxgiFormat::BC6H_Typeless => {}
                DxgiFormat::BC6H_UF16 => ImageAssetDataFormat::BC6H_UFloat,
                DxgiFormat::BC6H_SF16 => ImageAssetDataFormat::BC6H_SFloat,
                //DxgiFormat::BC7_Typeless => {}
                DxgiFormat::BC7_UNorm => ImageAssetDataFormat::BC7_Unorm_Linear,
                DxgiFormat::BC7_UNorm_sRGB => ImageAssetDataFormat::BC7_Unorm_Srgb,
                _ => unimplemented!(),
            }
        } else {
            unimplemented!();
        };

        let width = dds.get_width();
        let height = dds.get_height();
        let array_layer_count = dds.get_num_array_layers();
        let mip_level_count = dds.get_num_mipmap_levels();

        if dds.get_depth() != 1 {
            unimplemented!("DDS importer only supports image depth = 1");
        }

        log::trace!(
            "w: {} h: {} layers: {} mips: {} format: {:?} dxgi_format: {:?} d3d_format: {:?}",
            width,
            height,
            array_layer_count,
            mip_level_count,
            format,
            dds.get_dxgi_format(),
            dds.get_d3d_format()
        );
        //println!("Import DDS texture: {:?}", dds);

        let mut layers_asset_data = Vec::with_capacity(array_layer_count as usize);
        for layer_index in 0..array_layer_count {
            let layer = dds.get_data(0).unwrap();

            let mut current_mipmap_size_bytes = dds.get_main_texture_size().unwrap() as usize;
            let min_mipmap_size_bytes = dds.get_min_mipmap_size_in_bytes() as usize;
            let mut offset_bytes = 0_usize;

            let mut mip_width = width;
            let mut mip_height = height;

            let mut mip_levels_asset_data = Vec::with_capacity(mip_level_count as usize);
            for mip_index in 0..mip_level_count {
                let mip_data: Vec<u8> = layer
                    [offset_bytes..(offset_bytes + current_mipmap_size_bytes)]
                    .iter()
                    .copied()
                    .collect();
                log::trace!(
                    "Gathered mip data {} {} {}",
                    layer_index,
                    mip_index,
                    mip_data.len()
                );

                mip_levels_asset_data.push(ImageAssetDataMipLevel {
                    width: mip_width,
                    height: mip_height,
                    bytes: mip_data,
                });

                offset_bytes += current_mipmap_size_bytes;
                current_mipmap_size_bytes /= 4;
                if current_mipmap_size_bytes < min_mipmap_size_bytes {
                    current_mipmap_size_bytes = min_mipmap_size_bytes;
                }

                mip_width /= 2;
                mip_height /= 2;
            }

            layers_asset_data.push(ImageAssetDataLayer {
                mip_levels: mip_levels_asset_data,
            });
        }

        //
        // Create import data
        //
        let import_data = {
            let mut import_object =
                GpuCompressedImageImportedDataRecord::new_single_object(schema_set).unwrap();
            let mut import_data_container =
                DataContainerMut::new_single_object(&mut import_object, schema_set);
            let x = GpuCompressedImageImportedDataRecord::default();

            x.height().set(&mut import_data_container, height).unwrap();
            x.width().set(&mut import_data_container, width).unwrap();
            x.format()
                .set(
                    &mut import_data_container,
                    match format {
                        ImageAssetDataFormat::RGBA32_Linear => {
                            GpuImageAssetDataFormatEnum::RGBA32_Linear
                        }
                        ImageAssetDataFormat::RGBA32_Srgb => {
                            GpuImageAssetDataFormatEnum::RGBA32_Srgb
                        }
                        ImageAssetDataFormat::Basis_Linear => {
                            GpuImageAssetDataFormatEnum::Basis_Linear
                        }
                        ImageAssetDataFormat::Basis_Srgb => GpuImageAssetDataFormatEnum::Basis_Srgb,
                        ImageAssetDataFormat::BC1_UNorm_Linear => {
                            GpuImageAssetDataFormatEnum::BC1_UNorm_Linear
                        }
                        ImageAssetDataFormat::BC1_UNorm_Srgb => {
                            GpuImageAssetDataFormatEnum::BC1_UNorm_Srgb
                        }
                        ImageAssetDataFormat::BC2_UNorm_Linear => {
                            GpuImageAssetDataFormatEnum::BC2_UNorm_Linear
                        }
                        ImageAssetDataFormat::BC2_UNorm_Srgb => {
                            GpuImageAssetDataFormatEnum::BC2_UNorm_Srgb
                        }
                        ImageAssetDataFormat::BC3_UNorm_Linear => {
                            GpuImageAssetDataFormatEnum::BC3_UNorm_Linear
                        }
                        ImageAssetDataFormat::BC3_UNorm_Srgb => {
                            GpuImageAssetDataFormatEnum::BC3_UNorm_Srgb
                        }
                        ImageAssetDataFormat::BC4_UNorm => GpuImageAssetDataFormatEnum::BC4_UNorm,
                        ImageAssetDataFormat::BC4_SNorm => GpuImageAssetDataFormatEnum::BC4_SNorm,
                        ImageAssetDataFormat::BC5_UNorm => GpuImageAssetDataFormatEnum::BC5_UNorm,
                        ImageAssetDataFormat::BC5_SNorm => GpuImageAssetDataFormatEnum::BC5_SNorm,
                        ImageAssetDataFormat::BC6H_UFloat => {
                            GpuImageAssetDataFormatEnum::BC6H_UFloat
                        }
                        ImageAssetDataFormat::BC6H_SFloat => {
                            GpuImageAssetDataFormatEnum::BC6H_SFloat
                        }
                        ImageAssetDataFormat::BC7_Unorm_Linear => {
                            GpuImageAssetDataFormatEnum::BC7_Unorm_Linear
                        }
                        ImageAssetDataFormat::BC7_Unorm_Srgb => {
                            GpuImageAssetDataFormatEnum::BC7_Unorm_Srgb
                        }
                    },
                )
                .unwrap();

            for layer in layers_asset_data {
                let layer_entry = x.data_layers().add_entry(&mut import_data_container);
                let layer_record = x.data_layers().entry(layer_entry);

                for mip_level in layer.mip_levels {
                    let mip_level_entry = layer_record
                        .mip_levels()
                        .add_entry(&mut import_data_container);

                    let mip_record = layer_record.mip_levels().entry(mip_level_entry);
                    mip_record
                        .width()
                        .set(&mut import_data_container, mip_level.width)
                        .unwrap();
                    mip_record
                        .height()
                        .set(&mut import_data_container, mip_level.height)
                        .unwrap();
                    mip_record
                        .bytes()
                        .set(&mut import_data_container, mip_level.bytes)
                        .unwrap();
                }
            }

            import_object
        };

        //
        // Create the default asset
        //
        let default_asset = {
            let default_asset_object =
                GpuCompressedImageAssetRecord::new_single_object(schema_set).unwrap();

            // no fields to set

            default_asset_object
        };

        //
        // Return the created objects
        //
        let mut imported_objects = HashMap::default();
        imported_objects.insert(
            None,
            ImportedImportable {
                file_references: Default::default(),
                import_data: Some(import_data),
                default_asset: Some(default_asset),
            },
        );
        imported_objects
    }
}

#[derive(Hash, Serialize, Deserialize)]
pub struct GpuCompressedImageJobInput {
    pub asset_id: ObjectId,
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
        dependency_data: &HashMap<ObjectId, SingleObject>,
        job_api: &dyn JobApi,
    ) -> GpuCompressedImageJobOutput {
        //
        // Read asset properties
        //
        // let data_container = DataContainer::new_dataset(data_set, schema_set, input.asset_id);
        // let x = GpuImageAssetRecord::default();
        // let basis_compression = x.basis_compression().get(&data_container).unwrap();
        // let color_space = match x.color_space().get(&data_container).unwrap() {
        //     GpuImageColorSpaceEnum::Srgb => ImageAssetColorSpaceConfig::Srgb,
        //     GpuImageColorSpaceEnum::Linear => ImageAssetColorSpaceConfig::Linear,
        // };
        // let mip_generation = match x.mip_generation().get(&data_container).unwrap() {
        //     GpuImageMipGenerationEnum::NoMips => ImageAssetMipGeneration::NoMips,
        //     GpuImageMipGenerationEnum::Precomputed => ImageAssetMipGeneration::Precomupted,
        //     GpuImageMipGenerationEnum::Runtime => ImageAssetMipGeneration::Runtime,
        // };
        //
        // let format_config = if basis_compression {
        //     let compression_type = match x
        //         .basis_compression_settings()
        //         .compression_type()
        //         .get(&data_container)
        //         .unwrap()
        //     {
        //         GpuImageBasisCompressionTypeEnum::Uastc => ImageAssetBasisCompressionType::Uastc,
        //         GpuImageBasisCompressionTypeEnum::Etc1S => ImageAssetBasisCompressionType::Etc1S,
        //     };
        //     let quality = x
        //         .basis_compression_settings()
        //         .quality()
        //         .get(&data_container)
        //         .unwrap();
        //
        //     ImageAssetDataFormatConfig::BasisCompressed(ImageAssetBasisCompressionSettings {
        //         compression_type,
        //         quality,
        //     })
        // } else {
        //     ImageAssetDataFormatConfig::Uncompressed
        // };

        //
        // Read imported data
        //
        let imported_data = &dependency_data[&input.asset_id];
        let data_container = DataContainer::new_single_object(&imported_data, schema_set);
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

        let layer_entries = x.data_layers().resolve_entries(&data_container);
        let payload = if layer_entries.is_empty() {
            ImageAssetDataPayload::SingleBuffer(ImageAssetDataPayloadSingleBuffer {
                buffer: x.data_single_buffer().get(&data_container).unwrap().clone(),
            })
        } else {
            let mut layers = Vec::default();
            for &layer_entry in layer_entries.into_iter() {
                let layer = x.data_layers().entry(layer_entry);
                let mip_level_entries = layer.mip_levels().resolve_entries(&data_container);
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
        asset_id: ObjectId,
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
