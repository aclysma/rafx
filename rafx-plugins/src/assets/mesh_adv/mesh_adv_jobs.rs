pub use super::*;
use glam::Vec3;
use rafx::api::RafxResourceType;

use crate::features::mesh_adv::{MeshVertexFull, MeshVertexPosition};
use crate::schema::*;
use hydrate_pipeline::{
    AssetId, BuilderContext, BuilderRegistryBuilder, DataContainerRef,
    EnumerateDependenciesContext, ImporterRegistryBuilder, JobEnumeratedDependencies, JobInput,
    JobOutput, JobProcessor, JobProcessorRegistryBuilder, PipelineResult, RecordAccessor,
    RunContext, SchemaLinker,
};
use hydrate_pipeline::{AssetPlugin, Builder};
use rafx::assets::PushBuffer;
use rafx::rafx_visibility::{PolygonSoup, PolygonSoupIndex, VisibleBounds};
use serde::{Deserialize, Serialize};
use type_uuid::TypeUuid;
use uuid::Uuid;

#[derive(Hash, Serialize, Deserialize)]
pub struct MeshAdvMaterialJobInput {
    pub asset_id: AssetId,
}
impl JobInput for MeshAdvMaterialJobInput {}

#[derive(Serialize, Deserialize)]
pub struct MeshAdvMaterialJobOutput {}
impl JobOutput for MeshAdvMaterialJobOutput {}

#[derive(Default, TypeUuid)]
#[uuid = "d28004fa-6eb7-4110-8a17-10d42d92a956"]
pub struct MeshAdvMaterialJobProcessor;

impl JobProcessor for MeshAdvMaterialJobProcessor {
    type InputT = MeshAdvMaterialJobInput;
    type OutputT = MeshAdvMaterialJobOutput;

    fn version(&self) -> u32 {
        1
    }

    fn enumerate_dependencies(
        &self,
        _context: EnumerateDependenciesContext<Self::InputT>,
    ) -> PipelineResult<JobEnumeratedDependencies> {
        // No dependencies
        Ok(JobEnumeratedDependencies::default())
    }

    fn run(
        &self,
        context: RunContext<Self::InputT>,
    ) -> PipelineResult<MeshAdvMaterialJobOutput> {
        //
        // Read asset data
        //
        let data_container = DataContainerRef::from_dataset(
            context.data_set,
            context.schema_set,
            context.input.asset_id,
        );
        let x = MeshAdvMaterialAssetAccessor::default();

        let base_color_factor = x.base_color_factor().get_vec4(data_container)?;
        let emissive_factor = x.emissive_factor().get_vec3(data_container)?;

        let metallic_factor = x.metallic_factor().get(data_container)?;
        let roughness_factor = x.roughness_factor().get(data_container)?;
        let normal_texture_scale = x.normal_texture_scale().get(data_container)?;

        let color_texture = x.color_texture().get(data_container)?;
        let metallic_roughness_texture = x.metallic_roughness_texture().get(data_container)?;
        let normal_texture = x.normal_texture().get(data_container)?;
        let emissive_texture = x.emissive_texture().get(data_container)?;
        let shadow_method = x.shadow_method().get(data_container)?;
        let blend_method = x.blend_method().get(data_container)?;

        let alpha_threshold = x.alpha_threshold().get(data_container)?;
        let backface_culling = x.backface_culling().get(data_container)?;
        let color_texture_has_alpha_channel =
            x.color_texture_has_alpha_channel().get(data_container)?;

        //
        // Create the processed data
        //
        let material_data = MeshAdvMaterialData {
            base_color_factor,
            emissive_factor,
            metallic_factor,
            roughness_factor,
            normal_texture_scale,
            has_base_color_texture: !color_texture.is_null(),
            base_color_texture_has_alpha_channel: color_texture_has_alpha_channel,
            has_metallic_roughness_texture: !metallic_roughness_texture.is_null(),
            has_normal_texture: !normal_texture.is_null(),
            has_emissive_texture: !emissive_texture.is_null(),
            shadow_method: shadow_method.into(),
            blend_method: blend_method.into(),
            alpha_threshold,
            backface_culling,
        };

        context.produce_default_artifact_with_handles(
            context.input.asset_id,
            |handle_factory| {
                let material_asset = handle_factory.make_handle_to_default_artifact(
                    AssetId::from_uuid(Uuid::parse_str("07ab9227-432d-49c8-8899-146acd803235")?),
                );

                let color_texture_handle = if !color_texture.is_null() {
                    Some(handle_factory.make_handle_to_default_artifact(color_texture))
                } else {
                    None
                };

                let metallic_roughness_texture_handle = if !metallic_roughness_texture.is_null() {
                    Some(handle_factory.make_handle_to_default_artifact(metallic_roughness_texture))
                } else {
                    None
                };

                let normal_texture_handle = if !normal_texture.is_null() {
                    Some(handle_factory.make_handle_to_default_artifact(normal_texture))
                } else {
                    None
                };

                let emissive_texture_handle = if !emissive_texture.is_null() {
                    Some(handle_factory.make_handle_to_default_artifact(emissive_texture))
                } else {
                    None
                };

                let processed_data = MeshMaterialAdvAssetData {
                    material_data,
                    material_asset,
                    color_texture: color_texture_handle,
                    metallic_roughness_texture: metallic_roughness_texture_handle,
                    normal_texture: normal_texture_handle,
                    emissive_texture: emissive_texture_handle,
                };

                Ok(processed_data)
            },
        )?;

        //
        // Serialize and return
        //
        //job_system::produce_asset(job_api, input.asset_id, processed_data);

        Ok(MeshAdvMaterialJobOutput {})
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "02f17f4e-8df2-4b79-95cf-d2ee62e92a01"]
pub struct MeshAdvMaterialBuilder {}

impl Builder for MeshAdvMaterialBuilder {
    fn asset_type(&self) -> &'static str {
        MeshAdvMaterialAssetAccessor::schema_name()
    }

    fn start_jobs(
        &self,
        context: BuilderContext,
    ) -> PipelineResult<()> {
        //Future: Might produce jobs per-platform
        context.enqueue_job::<MeshAdvMaterialJobProcessor>(
            context.data_set,
            context.schema_set,
            context.job_api,
            MeshAdvMaterialJobInput {
                asset_id: context.asset_id,
            },
        )?;
        Ok(())
    }
}

fn try_cast_u8_slice<T: Copy + 'static>(data: &[u8]) -> Option<&[T]> {
    if data.len() % std::mem::size_of::<T>() != 0 {
        return None;
    }

    let ptr = data.as_ptr() as *const T;
    if ptr as usize % std::mem::align_of::<T>() != 0 {
        return None;
    }

    let casted: &[T] =
        unsafe { std::slice::from_raw_parts(ptr, data.len() / std::mem::size_of::<T>()) };

    Some(casted)
}

#[derive(Hash, Serialize, Deserialize)]
pub struct MeshAdvMeshPreprocessJobInput {
    pub asset_id: AssetId,
}
impl JobInput for MeshAdvMeshPreprocessJobInput {}

#[derive(Serialize, Deserialize)]
pub struct MeshAdvMeshPreprocessJobOutput {}
impl JobOutput for MeshAdvMeshPreprocessJobOutput {}

#[derive(Default, TypeUuid)]
#[uuid = "d1a87176-09b5-4722-802e-60012653966d"]
pub struct MeshAdvMeshJobProcessor;

impl JobProcessor for MeshAdvMeshJobProcessor {
    type InputT = MeshAdvMeshPreprocessJobInput;
    type OutputT = MeshAdvMeshPreprocessJobOutput;

    fn version(&self) -> u32 {
        1
    }

    fn enumerate_dependencies(
        &self,
        context: EnumerateDependenciesContext<Self::InputT>,
    ) -> PipelineResult<JobEnumeratedDependencies> {
        // No dependencies
        Ok(JobEnumeratedDependencies {
            import_data: vec![context.input.asset_id],
            upstream_jobs: Vec::default(),
        })
    }

    fn run(
        &self,
        context: RunContext<Self::InputT>,
    ) -> PipelineResult<MeshAdvMeshPreprocessJobOutput> {
        //
        // Read asset data
        //
        let asset_data = context.asset::<MeshAdvMeshAssetReader>(context.input.asset_id)?;
        let mut materials = Vec::default();
        for entry in asset_data.material_slots().resolve_entries()?.into_iter() {
            let entry = asset_data.material_slots().entry(*entry).get()?;
            materials.push(entry);
        }

        //
        // Read import data
        //
        let imported_data =
            context.imported_data::<MeshAdvMeshImportedDataReader>(context.input.asset_id)?;

        let mut all_positions = Vec::<glam::Vec3>::with_capacity(1024);
        let mut all_position_indices = Vec::<u32>::with_capacity(8192);

        let mut all_vertices_full = PushBuffer::new(16384);
        let mut all_vertices_position = PushBuffer::new(16384);
        let mut all_indices = PushBuffer::new(16384);

        let mut mesh_part_data = Vec::default();
        for entry in imported_data.mesh_parts().resolve_entries()?.into_iter() {
            let entry = imported_data.mesh_parts().entry(*entry);

            //
            // Get strongly typed slices of all input data for this mesh part
            //
            let positions_field_reader = entry.positions();
            let positions_bytes = positions_field_reader.get()?;
            let positions = try_cast_u8_slice::<[f32; 3]>(positions_bytes)
                .ok_or("Could not cast due to alignment")?;

            let normal_field_reader = entry.normals();
            let normals_bytes = normal_field_reader.get()?;
            let normals = try_cast_u8_slice::<[f32; 3]>(normals_bytes)
                .ok_or("Could not cast due to alignment")?;

            let tex_coords_field_reader = entry.texture_coordinates();
            let tex_coords_bytes = tex_coords_field_reader.get()?;
            let tex_coords = try_cast_u8_slice::<[f32; 2]>(tex_coords_bytes)
                .ok_or("Could not cast due to alignment")?;

            let indices_field_reader = entry.indices();
            let indices_bytes = indices_field_reader.get()?;
            let part_indices =
                try_cast_u8_slice::<u32>(indices_bytes).ok_or("Could not cast due to alignment")?;

            //
            // Part data which mostly contains offsets in the buffers for this part
            //
            let part_data = mesh_util::process_mesh_part(
                part_indices,
                positions,
                normals,
                tex_coords,
                &mut all_vertices_full,
                &mut all_vertices_position,
                &mut all_indices,
            );

            mesh_part_data.push(part_data);

            //
            // Positions and indices for the visibility system
            //
            for index in part_indices {
                all_position_indices.push(*index as u32);
            }

            for i in 0..positions.len() {
                all_positions.push(Vec3::new(positions[i][0], positions[i][1], positions[i][2]));
            }
        }

        //
        // Vertex Full Buffer
        //
        let vertex_buffer_full_artifact_id = if !all_vertices_full.is_empty() {
            context.produce_artifact(
                context.input.asset_id,
                Some("full"),
                MeshAdvBufferAssetData {
                    resource_type: RafxResourceType::VERTEX_BUFFER,
                    alignment: std::mem::size_of::<MeshVertexFull>() as u32,
                    data: all_vertices_full.into_data(),
                },
            )?
        } else {
            //TODO: This should probably just be an error
            Err("The mesh asset has no vertices")?
        };

        //
        // Vertex Position Buffer
        //
        let vertex_buffer_position_artifact_id = if !all_vertices_position.is_empty() {
            context.produce_artifact(
                context.input.asset_id,
                Some("position"),
                MeshAdvBufferAssetData {
                    resource_type: RafxResourceType::VERTEX_BUFFER,
                    alignment: std::mem::size_of::<MeshVertexPosition>() as u32,
                    data: all_vertices_position.into_data(),
                },
            )?
        } else {
            //TODO: This should probably just be an error
            Err("The mesh asset has no vertices")?
        };

        //
        // Index Buffer
        //
        let index_buffer_artifact_id = if !all_indices.is_empty() {
            context.produce_artifact(
                context.input.asset_id,
                Some("index"),
                MeshAdvBufferAssetData {
                    resource_type: RafxResourceType::INDEX_BUFFER,
                    alignment: std::mem::size_of::<u32>() as u32,
                    data: all_indices.into_data(),
                },
            )?
        } else {
            //TODO: This should probably just be an error
            Err("The mesh asset has no vertices")?
        };

        //
        // Mesh asset
        //
        context.produce_default_artifact_with_handles(
            context.input.asset_id,
            |handle_factory| {
                let mut mesh_parts = Vec::default();
                for (entry, part_data) in imported_data
                    .mesh_parts()
                    .resolve_entries()?
                    .into_iter()
                    .zip(mesh_part_data)
                {
                    let entry = imported_data.mesh_parts().entry(*entry);

                    let material_slot_index = entry.material_index().get()?;
                    let material_object_id = materials[material_slot_index as usize];

                    let material_handle =
                        handle_factory.make_handle_to_default_artifact(material_object_id);

                    mesh_parts.push(MeshAdvPartAssetData {
                        vertex_full_buffer_offset_in_bytes: part_data
                            .vertex_full_buffer_offset_in_bytes,
                        vertex_full_buffer_size_in_bytes: part_data
                            .vertex_full_buffer_size_in_bytes,
                        vertex_position_buffer_offset_in_bytes: part_data
                            .vertex_position_buffer_offset_in_bytes,
                        vertex_position_buffer_size_in_bytes: part_data
                            .vertex_position_buffer_size_in_bytes,
                        index_buffer_offset_in_bytes: part_data.index_buffer_offset_in_bytes,
                        index_buffer_size_in_bytes: part_data.index_buffer_size_in_bytes,
                        mesh_material: material_handle,
                        index_type: part_data.index_type,
                    })
                }

                let vertex_full_buffer =
                    handle_factory.make_handle_to_artifact(vertex_buffer_full_artifact_id);
                let vertex_position_buffer =
                    handle_factory.make_handle_to_artifact(vertex_buffer_position_artifact_id);
                let index_buffer = handle_factory.make_handle_to_artifact(index_buffer_artifact_id);

                let visible_bounds = PolygonSoup {
                    vertex_positions: all_positions,
                    index: PolygonSoupIndex::Indexed32(all_position_indices),
                };

                Ok(MeshAdvAssetData {
                    mesh_parts,
                    vertex_full_buffer,
                    vertex_position_buffer,
                    index_buffer,
                    visible_bounds: VisibleBounds::from(visible_bounds),
                })
            },
        )?;

        Ok(MeshAdvMeshPreprocessJobOutput {})
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "658b712f-e498-4c64-a26d-d83d775affb6"]
pub struct MeshAdvMeshBuilder {}

impl Builder for MeshAdvMeshBuilder {
    fn asset_type(&self) -> &'static str {
        MeshAdvMeshAssetAccessor::schema_name()
    }

    fn start_jobs(
        &self,
        context: BuilderContext,
    ) -> PipelineResult<()> {
        // Produce an intermediate with all data
        // Produce buffers for various vertex types
        // Some day I might want to look at the materials to decide what vertex buffers should exist

        context.enqueue_job::<MeshAdvMeshJobProcessor>(
            context.data_set,
            context.schema_set,
            context.job_api,
            MeshAdvMeshPreprocessJobInput {
                asset_id: context.asset_id,
            },
        )?;
        Ok(())
    }
}

#[derive(Hash, Serialize, Deserialize)]
pub struct MeshAdvModelJobInput {
    pub asset_id: AssetId,
}
impl JobInput for MeshAdvModelJobInput {}

#[derive(Serialize, Deserialize)]
pub struct MeshAdvModelJobOutput {}
impl JobOutput for MeshAdvModelJobOutput {}

#[derive(Default, TypeUuid)]
#[uuid = "16e92a68-1fb8-4b1e-af16-6af7dce34342"]
pub struct MeshAdvModelJobProcessor;

impl JobProcessor for MeshAdvModelJobProcessor {
    type InputT = MeshAdvModelJobInput;
    type OutputT = MeshAdvModelJobOutput;

    fn version(&self) -> u32 {
        1
    }

    fn enumerate_dependencies(
        &self,
        _context: EnumerateDependenciesContext<Self::InputT>,
    ) -> PipelineResult<JobEnumeratedDependencies> {
        // No dependencies
        Ok(JobEnumeratedDependencies::default())
    }

    fn run(
        &self,
        context: RunContext<Self::InputT>,
    ) -> PipelineResult<MeshAdvModelJobOutput> {
        context.produce_default_artifact_with_handles(
            context.input.asset_id,
            |handle_factory| {
                let data_container = DataContainerRef::from_dataset(
                    context.data_set,
                    context.schema_set,
                    context.input.asset_id,
                );
                let x = MeshAdvModelAssetAccessor::default();

                let mut lods = Vec::default();
                for entry in x.lods().resolve_entries(data_container)?.into_iter() {
                    let lod = x.lods().entry(*entry);
                    let mesh_handle = handle_factory
                        .make_handle_to_default_artifact(lod.mesh().get(data_container)?);

                    lods.push(ModelAdvAssetDataLod { mesh: mesh_handle });
                }

                Ok(ModelAdvAssetData { lods })
            },
        )?;

        Ok(MeshAdvModelJobOutput {})
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "1190eda4-e0c7-4851-ba1e-0ba56d1dc384"]
pub struct MeshAdvModelBuilder {}

impl Builder for MeshAdvModelBuilder {
    fn asset_type(&self) -> &'static str {
        MeshAdvModelAssetAccessor::schema_name()
    }

    fn start_jobs(
        &self,
        context: BuilderContext,
    ) -> PipelineResult<()> {
        //let data_container = DataContainerRef::from_dataset(data_set, schema_set, asset_id);
        //let x = MeshAdvModelAssetAccessor::default();

        //Future: Might produce jobs per-platform
        context.enqueue_job::<MeshAdvModelJobProcessor>(
            context.data_set,
            context.schema_set,
            context.job_api,
            MeshAdvModelJobInput {
                asset_id: context.asset_id,
            },
        )?;
        Ok(())
    }
}

#[derive(Hash, Serialize, Deserialize)]
pub struct MeshAdvPrefabJobInput {
    pub asset_id: AssetId,
}
impl JobInput for MeshAdvPrefabJobInput {}

#[derive(Serialize, Deserialize)]
pub struct MeshAdvPrefabJobOutput {}
impl JobOutput for MeshAdvPrefabJobOutput {}

#[derive(Default, TypeUuid)]
#[uuid = "a11b7361-33ae-4361-905b-fe25d2ac389e"]
pub struct MeshAdvPrefabJobProcessor;

impl JobProcessor for MeshAdvPrefabJobProcessor {
    type InputT = MeshAdvPrefabJobInput;
    type OutputT = MeshAdvPrefabJobOutput;

    fn version(&self) -> u32 {
        1
    }

    fn enumerate_dependencies(
        &self,
        context: EnumerateDependenciesContext<Self::InputT>,
    ) -> PipelineResult<JobEnumeratedDependencies> {
        // No dependencies
        Ok(JobEnumeratedDependencies {
            import_data: vec![context.input.asset_id],
            upstream_jobs: Default::default(),
        })
    }

    fn run(
        &self,
        context: RunContext<Self::InputT>,
    ) -> PipelineResult<MeshAdvPrefabJobOutput> {
        //
        // Read import data
        //
        let imported_data =
            context.imported_data::<MeshAdvPrefabImportDataReader>(context.input.asset_id)?;

        let json_str = imported_data.json_data().get()?;
        let json_format: HydrateMeshAdvPrefabJsonFormat = serde_json::from_str(&json_str)
            .map_err(|x| format!("Blender Material Import error: {:?}", x))?;

        let file_references = context
            .data_set
            .resolve_all_file_references(context.input.asset_id)?;

        context.produce_default_artifact_with_handles(
            context.input.asset_id,
            |handle_factory| {
                let mut objects = Vec::with_capacity(json_format.objects.len());
                for json_object in json_format.objects {
                    let model = if let Some(json_model) = &json_object.model {
                        let model_object_id = file_references
                            .get(&json_model.model)
                            .ok_or("Could not find asset ID associated with path")?;
                        let model_handle =
                            handle_factory.make_handle_to_default_artifact(*model_object_id);

                        Some(PrefabAdvAssetDataObjectModel {
                            model: model_handle,
                        })
                    } else {
                        None
                    };

                    let light = if let Some(light) = &json_object.light {
                        let spot = light
                            .spot
                            .as_ref()
                            .map(|x| PrefabAdvAssetDataObjectLightSpot {
                                inner_angle: x.inner_angle,
                                outer_angle: x.outer_angle,
                            });

                        let range = if light.cutoff_distance.unwrap_or(-1.0) < 0.0 {
                            None
                        } else {
                            light.cutoff_distance
                        };
                        Some(PrefabAdvAssetDataObjectLight {
                            color: light.color.into(),
                            kind: light.kind.into(),
                            intensity: light.intensity,
                            range,
                            spot,
                        })
                    } else {
                        None
                    };

                    let transform = PrefabAdvAssetDataObjectTransform {
                        position: json_object.transform.position.into(),
                        rotation: json_object.transform.rotation.into(),
                        scale: json_object.transform.scale.into(),
                    };

                    objects.push(PrefabAdvAssetDataObject {
                        transform,
                        model,
                        light,
                    });
                }

                Ok(PrefabAdvAssetData { objects })
            },
        )?;

        Ok(MeshAdvPrefabJobOutput {})
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "e5e3879c-5ff6-4823-b53d-a209a1fed82f"]
pub struct MeshAdvPrefabBuilder {}

impl Builder for MeshAdvPrefabBuilder {
    fn asset_type(&self) -> &'static str {
        MeshAdvPrefabAssetAccessor::schema_name()
    }

    fn start_jobs(
        &self,
        context: BuilderContext,
    ) -> PipelineResult<()> {
        //let data_container = DataContainerRef::from_dataset(data_set, schema_set, asset_id);
        //let x = MeshAdvPrefabAssetAccessor::default();

        //Future: Might produce jobs per-platform
        context.enqueue_job::<MeshAdvPrefabJobProcessor>(
            context.data_set,
            context.schema_set,
            context.job_api,
            MeshAdvPrefabJobInput {
                asset_id: context.asset_id,
            },
        )?;
        Ok(())
    }
}

pub struct MeshAdvAssetPlugin;

impl AssetPlugin for MeshAdvAssetPlugin {
    fn setup(
        _schema_linker: &mut SchemaLinker,
        _importer_registry: &mut ImporterRegistryBuilder,
        builder_registry: &mut BuilderRegistryBuilder,
        job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        builder_registry.register_handler::<MeshAdvMaterialBuilder>();
        job_processor_registry.register_job_processor::<MeshAdvMaterialJobProcessor>();

        builder_registry.register_handler::<MeshAdvMeshBuilder>();
        job_processor_registry.register_job_processor::<MeshAdvMeshJobProcessor>();

        builder_registry.register_handler::<MeshAdvModelBuilder>();
        job_processor_registry.register_job_processor::<MeshAdvModelJobProcessor>();

        builder_registry.register_handler::<MeshAdvPrefabBuilder>();
        job_processor_registry.register_job_processor::<MeshAdvPrefabJobProcessor>();
    }
}
