use crate::assets::anim::{
    AnimAssetData, AnimClip, AnimInterpolationMode, Bone, BoneChannelGroup, BoneChannelQuat,
    BoneChannelVec3, Skeleton,
};
use crate::schema::{BlenderAnimAssetRecord, BlenderAnimImportedDataRecord};
use fnv::FnvHashMap;
use hydrate_base::hashing::HashMap;
use hydrate_base::ObjectId;
use hydrate_data::{
    DataContainer, DataContainerMut, DataSet, Field, PropertyPath, Record, SchemaSet, SingleObject,
};
use hydrate_model::{
    job_system, BuilderRegistryBuilder, ImportableObject, ImportedImportable, ImporterRegistry,
    ImporterRegistryBuilder, JobApi, JobEnumeratedDependencies, JobInput, JobOutput, JobProcessor,
    JobProcessorRegistryBuilder, ReferencedSourceFile, ScannedImportable, SchemaLinker,
};
use rafx::api::{RafxError, RafxResult};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::io::Read;
use std::path::Path;
use type_uuid::*;

#[allow(dead_code)]
fn parse_interpolation_mode(mode: &str) -> RafxResult<AnimInterpolationMode> {
    Ok(match mode.to_lowercase().as_str() {
        "linear" => AnimInterpolationMode::Linear,
        _ => {
            return Err(RafxError::StringError(format!(
                "Cannot parse AnimInterpolationMode {}",
                mode
            )))
        }
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SkeletonBoneJsonData {
    name: String,
    parent: String,
    position: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SkeletonJsonData {
    bones: Vec<SkeletonBoneJsonData>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActionChannelInterpolationJsonData {
    frame: u32,
    mode: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActionChannelVec3JsonData {
    min_frame: u32,
    max_frame: u32,
    interpolation: Vec<ActionChannelInterpolationJsonData>,
    values: Vec<[f32; 3]>,
}

impl TryInto<BoneChannelVec3> for &ActionChannelVec3JsonData {
    type Error = RafxError;

    fn try_into(self) -> Result<BoneChannelVec3, Self::Error> {
        //parse_interpolation_mode(self.interpolation)

        let values = self.values.iter().map(|&x| x.into()).collect();
        Ok(BoneChannelVec3 {
            min_frame: self.min_frame,
            max_frame: self.max_frame,
            values,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActionChannelVec4JsonData {
    min_frame: u32,
    max_frame: u32,
    interpolation: Vec<ActionChannelInterpolationJsonData>,
    values: Vec<[f32; 4]>,
}

impl TryInto<BoneChannelQuat> for &ActionChannelVec4JsonData {
    type Error = RafxError;

    fn try_into(self) -> Result<BoneChannelQuat, Self::Error> {
        //parse_interpolation_mode(self.interpolation)

        let values = self.values.iter().map(|&x| x.into()).collect();
        Ok(BoneChannelQuat {
            min_frame: self.min_frame,
            max_frame: self.max_frame,
            values,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ActionBoneChannelGroupJsonData {
    bone_name: String,
    position: Option<ActionChannelVec3JsonData>,
    rotation: Option<ActionChannelVec4JsonData>,
    scale: Option<ActionChannelVec3JsonData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ActionJsonData {
    name: String,
    bone_channel_groups: Vec<ActionBoneChannelGroupJsonData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AnimJsonData {
    skeleton: SkeletonJsonData,
    actions: Vec<ActionJsonData>,
}

// #[derive(TypeUuid, Serialize, Deserialize, Default, Clone, Debug)]
// #[uuid = "da73abc3-aaa9-447e-8726-e5e932383288"]
// pub struct AnimImporterOptions {}
//
// // The asset state is stored in this format using Vecs
// #[derive(TypeUuid, Serialize, Deserialize, Default, Clone, Debug)]
// #[uuid = "c995f022-8214-4bfe-a8ee-b4bff873901d"]
// pub struct AnimImporterStateStable {
//     anim_asset_uuid: Option<AssetUuid>,
// }
//
// impl From<AnimImporterStateUnstable> for AnimImporterStateStable {
//     fn from(other: AnimImporterStateUnstable) -> Self {
//         let mut stable = AnimImporterStateStable::default();
//         stable.anim_asset_uuid = other.anim_asset_uuid.clone();
//         stable
//     }
// }
//
// #[derive(Default)]
// pub struct AnimImporterStateUnstable {
//     anim_asset_uuid: Option<AssetUuid>,
// }
//
// impl From<AnimImporterStateStable> for AnimImporterStateUnstable {
//     fn from(other: AnimImporterStateStable) -> Self {
//         let mut unstable = AnimImporterStateUnstable::default();
//         unstable.anim_asset_uuid = other.anim_asset_uuid.clone();
//         unstable
//     }
// }
//
// #[derive(TypeUuid)]
// #[uuid = "fe509d69-62ed-40a1-badd-b45d2fbfce07"]
// pub struct BlenderAnimImporter;
// impl Importer for BlenderAnimImporter {
//     fn version_static() -> u32
//     where
//         Self: Sized,
//     {
//         2
//     }
//
//     fn version(&self) -> u32 {
//         Self::version_static()
//     }
//
//     type Options = AnimImporterOptions;
//
//     type State = AnimImporterStateStable;
//
//     /// Reads the given bytes and produces assets.
//     #[profiling::function]
//     fn import(
//         &self,
//         _op: &mut ImportOp,
//         source: &mut dyn Read,
//         _options: &Self::Options,
//         stable_state: &mut Self::State,
//     ) -> distill::importer::Result<ImporterValue> {
//         let mut unstable_state: AnimImporterStateUnstable = stable_state.clone().into();
//
//         //
//         // Assign an ID to this anim file if not already assigned
//         //
//         unstable_state.anim_asset_uuid = Some(
//             unstable_state
//                 .anim_asset_uuid
//                 .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes())),
//         );
//
//         // Read in the anim file
//         let anim_data: serde_json::Result<AnimJsonData> = serde_json::from_reader(source);
//         if let Err(err) = anim_data {
//             log::error!("anim Import error: {:?}", err);
//             return Err(Error::Boxed(Box::new(err)));
//         }
//
//         let anim_data = anim_data.unwrap();
//
//         let skeleton = parse_skeleton(&anim_data.skeleton).map_err(|e| e.to_string())?;
//
//         let mut clips = Vec::with_capacity(anim_data.actions.len());
//         for action in &anim_data.actions {
//             clips.push(parse_action(&skeleton, action).map_err(|e| e.to_string())?);
//         }
//
//         let asset_data = AnimAssetData { skeleton, clips };
//
//         let mut imported_assets = Vec::<ImportedAsset>::default();
//         imported_assets.push(ImportedAsset {
//             id: unstable_state.anim_asset_uuid.unwrap(),
//             search_tags: vec![],
//             build_deps: vec![],
//             load_deps: vec![],
//             build_pipeline: None,
//             asset_data: Box::new(asset_data),
//         });
//
//         *stable_state = unstable_state.into();
//
//         Ok(ImporterValue {
//             assets: imported_assets,
//         })
//     }
// }

fn try_add_bone(
    bone_data: &SkeletonBoneJsonData,
    bones: &mut Vec<Bone>,
    bone_index_lookup: &mut FnvHashMap<String, i16>,
) {
    let parent_index = bone_index_lookup.get(&bone_data.parent).copied();

    if !bone_data.parent.is_empty() && parent_index.is_none() {
        // Has parent, but parent wasn't added yet
        return;
    }

    let chain_depth = parent_index
        .map(|x| &bones[x as usize])
        .map(|x| x.chain_depth + 1)
        .unwrap_or(0);

    let bone_index = bones.len() as i16;
    bones.push(Bone {
        name: bone_data.name.clone(),
        parent: parent_index.unwrap_or(-1),
        chain_depth,
        position_rel: bone_data.position.into(),
        rotation_rel: bone_data.rotation.into(),
    });
    bone_index_lookup.insert(bone_data.name.clone(), bone_index);
}

fn parse_skeleton(skeleton_data: &SkeletonJsonData) -> RafxResult<Skeleton> {
    let mut bone_data_index_lookup = FnvHashMap::default();
    for (i, bone_data) in skeleton_data.bones.iter().enumerate() {
        if bone_data.name.is_empty() {
            Err("bone has empty name")?;
        }

        let old = bone_data_index_lookup.insert(bone_data.name.clone(), i);
        if old.is_some() {
            Err(format!("multiple bones with name {} found", bone_data.name))?;
        }
    }

    for bone_data in &skeleton_data.bones {
        if !bone_data.parent.is_empty() {
            if !bone_data_index_lookup.contains_key(&bone_data.parent) {
                Err(format!(
                    "cannot find parent bone {} for child bone {}",
                    bone_data.parent, bone_data.name
                ))?;
            }
        }
    }

    // This will construct a list of bones sorted by ascending chain_depth. It assumes that all
    // parents exist, and that there are no duplicate names
    let mut bones = Vec::with_capacity(skeleton_data.bones.len());
    let mut bone_index_lookup = FnvHashMap::default();
    loop {
        let bone_count = bones.len();
        for bone_data in &skeleton_data.bones {
            if !bone_index_lookup.contains_key(&bone_data.name) {
                try_add_bone(bone_data, &mut bones, &mut bone_index_lookup);
            }
        }

        if bone_count == bones.len() {
            break;
        }
    }

    if bones.len() != skeleton_data.bones.len() {
        let mut missing_bones = vec![];
        for bone in &skeleton_data.bones {
            if !bone_index_lookup.contains_key(&bone.name) {
                missing_bones.push(bone.name.clone());
            }
        }

        Err(format!("The following bones could not be added, likely there is a cycle in parent/child relationships: {:?}", missing_bones))?;
    }

    Ok(Skeleton { bones })
}

fn parse_action(
    skeleton: &Skeleton,
    action: &ActionJsonData,
) -> RafxResult<AnimClip> {
    let mut bone_channel_groups_lookup = FnvHashMap::default();
    for (i, bone_channel_group) in action.bone_channel_groups.iter().enumerate() {
        let old = bone_channel_groups_lookup.insert(&bone_channel_group.bone_name, i);
        assert!(old.is_none());
    }

    let mut bone_channel_groups = Vec::with_capacity(skeleton.bones.len());
    for bone in &skeleton.bones {
        if let Some(channel_group_index) = bone_channel_groups_lookup.get(&bone.name) {
            //TODO: CLONE IS TEMPORARY
            let mut channel_group = BoneChannelGroup::default();

            let json_channel_group_data = &action.bone_channel_groups[*channel_group_index];
            if let Some(position) = &json_channel_group_data.position {
                channel_group.position = Some(position.try_into()?);
            }

            if let Some(rotation) = &json_channel_group_data.rotation {
                channel_group.rotation = Some(rotation.try_into()?);
            }

            if let Some(scale) = &json_channel_group_data.scale {
                channel_group.scale = Some(scale.try_into()?);
            }

            bone_channel_groups.push(channel_group);
        } else {
            bone_channel_groups.push(BoneChannelGroup::default());
        }
    }

    Ok(AnimClip {
        name: action.name.clone(),
        bone_channel_groups,
    })
}

#[derive(TypeUuid, Default)]
#[uuid = "238792bf-7078-4675-9f4d-cf53305806c6"]
pub struct HydrateBlenderAnimImporter;

impl hydrate_model::Importer for HydrateBlenderAnimImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["blender_anim"]
    }

    fn scan_file(
        &self,
        path: &Path,
        schema_set: &SchemaSet,
        importer_registry: &ImporterRegistry,
    ) -> Vec<ScannedImportable> {
        //
        // Read the file
        //
        let json_str = std::fs::read_to_string(path).unwrap();
        let anim_data: serde_json::Result<AnimJsonData> = serde_json::from_str(&json_str);
        if let Err(err) = anim_data {
            panic!("anim Import error: {:?}", err);
            //return Err(Error::Boxed(Box::new(err)));
        }

        let asset_type = schema_set
            .find_named_type(BlenderAnimAssetRecord::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();
        let mut file_references: Vec<ReferencedSourceFile> = Default::default();
        vec![ScannedImportable {
            name: None,
            asset_type,
            file_references,
        }]
    }

    fn import_file(
        &self,
        path: &Path,
        importable_objects: &HashMap<Option<String>, ImportableObject>,
        schema_set: &SchemaSet,
    ) -> HashMap<Option<String>, ImportedImportable> {
        //
        // Read the file
        //
        let json_str = std::fs::read_to_string(path).unwrap();
        let anim_data: serde_json::Result<AnimJsonData> = serde_json::from_str(&json_str);
        if let Err(err) = anim_data {
            panic!("anim Import error: {:?}", err);
            //return Err(Error::Boxed(Box::new(err)));
        }

        //
        // Create the default asset
        //
        let default_asset = {
            let mut default_asset_object =
                BlenderAnimAssetRecord::new_single_object(schema_set).unwrap();
            // let mut default_asset_data_container =
            //     DataContainerMut::new_single_object(&mut default_asset_object, schema_set);
            // let x = BlenderAnimAssetRecord::default();

            // No fields to write
            default_asset_object
        };

        //
        // Create import data
        //
        let import_data = {
            let mut import_object =
                BlenderAnimImportedDataRecord::new_single_object(schema_set).unwrap();
            let mut import_data_container =
                DataContainerMut::new_single_object(&mut import_object, schema_set);
            let x = BlenderAnimImportedDataRecord::default();

            x.json_string()
                .set(&mut import_data_container, json_str)
                .unwrap();

            import_object
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
pub struct BlenderAnimJobInput {
    pub asset_id: ObjectId,
}
impl JobInput for BlenderAnimJobInput {}

#[derive(Serialize, Deserialize)]
pub struct BlenderAnimJobOutput {}
impl JobOutput for BlenderAnimJobOutput {}

#[derive(Default, TypeUuid)]
#[uuid = "e7ab8a6c-6d53-4c05-b3e3-eb286ff2042a"]
pub struct BlenderAnimJobProcessor;

impl JobProcessor for BlenderAnimJobProcessor {
    type InputT = BlenderAnimJobInput;
    type OutputT = BlenderAnimJobOutput;

    fn version(&self) -> u32 {
        1
    }

    fn enumerate_dependencies(
        &self,
        input: &BlenderAnimJobInput,
        data_set: &DataSet,
        schema_set: &SchemaSet,
    ) -> JobEnumeratedDependencies {
        // No dependencies
        JobEnumeratedDependencies {
            import_data: vec![input.asset_id],
            upstream_jobs: Default::default(),
        }
    }

    fn run(
        &self,
        input: &BlenderAnimJobInput,
        data_set: &DataSet,
        schema_set: &SchemaSet,
        dependency_data: &HashMap<ObjectId, SingleObject>,
        job_api: &dyn JobApi,
    ) -> BlenderAnimJobOutput {
        //
        // Read imported data
        //
        let imported_data = &dependency_data[&input.asset_id];
        let data_container = DataContainer::new_single_object(&imported_data, schema_set);
        let x = BlenderAnimImportedDataRecord::new(PropertyPath::default());

        let json_str = x.json_string().get(&data_container).unwrap();

        let anim_data: serde_json::Result<AnimJsonData> = serde_json::from_str(&json_str);
        if let Err(err) = anim_data {
            panic!("anim Import error: {:?}", err);
            //return Err(Error::Boxed(Box::new(err)));
        }

        let anim_data = anim_data.unwrap();

        let skeleton = parse_skeleton(&anim_data.skeleton)
            .map_err(|e| e.to_string())
            .unwrap();

        let mut clips = Vec::with_capacity(anim_data.actions.len());
        for action in &anim_data.actions {
            clips.push(
                parse_action(&skeleton, action)
                    .map_err(|e| e.to_string())
                    .unwrap(),
            );
        }

        job_system::produce_asset(job_api, input.asset_id, AnimAssetData { skeleton, clips });

        BlenderAnimJobOutput {}
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "77a09407-3ec8-440d-bd01-408b84b4516c"]
pub struct BlenderAnimBuilder {}

impl hydrate_model::Builder for BlenderAnimBuilder {
    fn asset_type(&self) -> &'static str {
        BlenderAnimAssetRecord::schema_name()
    }

    fn start_jobs(
        &self,
        asset_id: ObjectId,
        data_set: &DataSet,
        schema_set: &SchemaSet,
        job_api: &dyn JobApi,
    ) {
        //let data_container = DataContainer::new_dataset(data_set, schema_set, asset_id);
        //let x = BlenderAnimAssetRecord::default();

        //Future: Might produce jobs per-platform
        job_system::enqueue_job::<BlenderAnimJobProcessor>(
            data_set,
            schema_set,
            job_api,
            BlenderAnimJobInput { asset_id },
        );
    }
}

pub struct BlenderAnimAssetPlugin;

impl hydrate_model::AssetPlugin for BlenderAnimAssetPlugin {
    fn setup(
        schema_linker: &mut SchemaLinker,
        importer_registry: &mut ImporterRegistryBuilder,
        builder_registry: &mut BuilderRegistryBuilder,
        job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<HydrateBlenderAnimImporter>(schema_linker);
        builder_registry.register_handler::<BlenderAnimBuilder>(schema_linker);
        job_processor_registry.register_job_processor::<BlenderAnimJobProcessor>();
    }
}
