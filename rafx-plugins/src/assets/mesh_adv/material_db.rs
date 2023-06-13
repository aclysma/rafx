use crate::assets::mesh_adv::{MeshAdvBlendMethod, MeshAdvMaterialData, MeshMaterialAdvAsset};
use crossbeam_channel::{Receiver, Sender};
use fnv::FnvHashMap;
use rafx::api::{RafxBufferDef, RafxError, RafxMemoryUsage, RafxQueueType, RafxResourceType};
use rafx::assets::UploadAssetOp;
use rafx::base::slab::{RawSlab, RawSlabKey};
use rafx::framework::{
    BufferResource, DescriptorSetArc, DescriptorSetBindings, DescriptorSetLayoutResource,
    ImageViewResource, ResourceArc, ResourceContext,
};
use rafx::RafxResult;
use std::sync::Arc;

use crate::shaders::mesh_adv::mesh_adv_textured_frag;

pub struct MeshAdvMaterialDef {
    pub data: MeshAdvMaterialData,
    pub color_texture: Option<ResourceArc<ImageViewResource>>,
    pub metallic_roughness_texture: Option<ResourceArc<ImageViewResource>>,
    pub normal_texture: Option<ResourceArc<ImageViewResource>>,
    pub emissive_texture: Option<ResourceArc<ImageViewResource>>,
}

pub struct MeshAdvMaterialInner {
    material_data: MeshAdvMaterialData,
    material_key: RawSlabKey<MaterialEntry>,
    material_data_index: u32,
    drop_tx: Sender<RawSlabKey<MaterialEntry>>,
}

pub struct MeshAdvMaterial {
    inner: Arc<MeshAdvMaterialInner>,
}

impl Drop for MeshAdvMaterialInner {
    fn drop(&mut self) {
        // We explicitly destroy the material DB so it's safe to ignore failures here
        let _ = self.drop_tx.send(self.material_key);
    }
}

impl MeshAdvMaterial {
    pub fn data(&self) -> &MeshAdvMaterialData {
        &self.inner.material_data
    }

    pub fn material_data_index(&self) -> u32 {
        self.inner.material_data_index
    }
}

struct MaterialEntry {
    data_key: RawSlabKey<MeshAdvMaterialData>,
    color_texture_key: Option<RawSlabKey<MaterialTextureMeta>>,
    metallic_roughness_texture_key: Option<RawSlabKey<MaterialTextureMeta>>,
    normal_texture_key: Option<RawSlabKey<MaterialTextureMeta>>,
    emissive_texture_key: Option<RawSlabKey<MaterialTextureMeta>>,
}

pub struct MaterialTextureMeta {
    image: ResourceArc<ImageViewResource>,
    ref_count: u32,
}

pub struct MaterialDB {
    material_entries: RawSlab<MaterialEntry>,

    material_data: RawSlab<MeshAdvMaterialData>,

    image_views: RawSlab<MaterialTextureMeta>,
    image_view_lookup: FnvHashMap<ResourceArc<ImageViewResource>, RawSlabKey<MaterialTextureMeta>>,

    drop_tx: Sender<RawSlabKey<MaterialEntry>>,
    drop_rx: Receiver<RawSlabKey<MaterialEntry>>,
}

impl MaterialDB {
    pub fn new() -> MaterialDB {
        let (drop_tx, drop_rx) = crossbeam_channel::unbounded();

        MaterialDB {
            material_entries: RawSlab::default(),
            material_data: RawSlab::default(),
            image_views: RawSlab::default(),
            image_view_lookup: Default::default(),
            drop_tx,
            drop_rx,
        }
    }

    pub fn add_material(
        &mut self,
        def: MeshAdvMaterialDef,
    ) -> MeshAdvMaterial {
        let data_key = self.material_data.allocate(def.data.clone());
        let color_texture_key = def.color_texture.map(|x| self.add_texture_ref(x));
        let metallic_roughness_texture_key = def
            .metallic_roughness_texture
            .map(|x| self.add_texture_ref(x));
        let normal_texture_key = def.normal_texture.map(|x| self.add_texture_ref(x));
        let emissive_texture_key = def.emissive_texture.map(|x| self.add_texture_ref(x));

        let entry = MaterialEntry {
            data_key,
            color_texture_key,
            metallic_roughness_texture_key,
            normal_texture_key,
            emissive_texture_key,
        };

        let material_key = self.material_entries.allocate(entry);
        let inner = MeshAdvMaterialInner {
            material_data: def.data,
            material_key,
            material_data_index: data_key.index(),
            drop_tx: self.drop_tx.clone(),
        };

        MeshAdvMaterial {
            inner: Arc::new(inner),
        }
    }

    fn do_remove_material(
        material_key: RawSlabKey<MaterialEntry>,
        material_entries: &mut RawSlab<MaterialEntry>,
        material_data: &mut RawSlab<MeshAdvMaterialData>,
        image_views: &mut RawSlab<MaterialTextureMeta>,
        image_view_lookup: &mut FnvHashMap<
            ResourceArc<ImageViewResource>,
            RawSlabKey<MaterialTextureMeta>,
        >,
    ) {
        let entry = material_entries.get(material_key).unwrap();
        let color_texture_key = entry.color_texture_key.clone();
        let metallic_roughness_texture_key = entry.metallic_roughness_texture_key.clone();
        let normal_texture_key = entry.normal_texture_key.clone();
        let emissive_texture_key = entry.emissive_texture_key.clone();

        material_data.free(entry.data_key);

        if let Some(key) = color_texture_key {
            Self::remove_texture_ref(key, image_views, image_view_lookup)
        }

        if let Some(key) = metallic_roughness_texture_key {
            Self::remove_texture_ref(key, image_views, image_view_lookup)
        }

        if let Some(key) = normal_texture_key {
            Self::remove_texture_ref(key, image_views, image_view_lookup)
        }

        if let Some(key) = emissive_texture_key {
            Self::remove_texture_ref(key, image_views, image_view_lookup)
        }

        material_entries.free(material_key);
    }

    fn add_texture_ref(
        &mut self,
        image_view: ResourceArc<ImageViewResource>,
    ) -> RawSlabKey<MaterialTextureMeta> {
        if let Some(&key) = self.image_view_lookup.get(&image_view) {
            self.image_views.get_mut(key).unwrap().ref_count += 1;
            key
        } else {
            let key = self.image_views.allocate(MaterialTextureMeta {
                ref_count: 1,
                image: image_view.clone(),
            });
            self.image_view_lookup.insert(image_view, key);
            assert!(key.index() < 768);
            key
        }
    }

    fn remove_texture_ref(
        key: RawSlabKey<MaterialTextureMeta>,
        image_views: &mut RawSlab<MaterialTextureMeta>,
        image_view_lookup: &mut FnvHashMap<
            ResourceArc<ImageViewResource>,
            RawSlabKey<MaterialTextureMeta>,
        >,
    ) {
        let meta = image_views.get_mut(key).unwrap();
        if meta.ref_count > 1 {
            meta.ref_count -= 1;
        } else {
            image_view_lookup.remove(&meta.image);
            image_views.free(key);
        }
    }

    fn create_all_materials_buffer(
        &self,
        resource_context: &ResourceContext,
    ) -> RafxResult<ResourceArc<BufferResource>> {
        use mesh_adv_textured_frag::MaterialDbEntryBuffer;

        let resource_allocator = resource_context.create_dyn_resource_allocator_set();

        let max_materials = self.material_entries.storage_size();
        let all_materials_buffer_size_bytes =
            std::mem::size_of::<MaterialDbEntryBuffer>() * max_materials;

        let material_data_sbo =
            resource_context
                .device_context()
                .create_buffer(&RafxBufferDef {
                    size: all_materials_buffer_size_bytes as u64,
                    alignment: 256,
                    memory_usage: RafxMemoryUsage::CpuToGpu,
                    queue_type: RafxQueueType::Graphics,
                    //DX12TODO: Does not need to be BUFFER_READ_WRITE for other backends
                    resource_type: RafxResourceType::BUFFER_READ_WRITE,
                    ..Default::default()
                })?;
        material_data_sbo.set_debug_name("MeshAdv Material Data");

        let mapped = material_data_sbo.map_buffer()?;
        let all_materials = unsafe {
            std::slice::from_raw_parts_mut(
                mapped as *mut mesh_adv_textured_frag::MaterialDbEntryBuffer,
                max_materials,
            )
        };

        for (key, entry) in self.material_entries.iter() {
            let material = &mut all_materials[key.index() as usize];
            let material_data = self.material_data.get(entry.data_key).unwrap();
            let color_texture = entry
                .color_texture_key
                .map(|x| x.index() as i32)
                .unwrap_or(-1);
            let metallic_roughness_texture = entry
                .metallic_roughness_texture_key
                .map(|x| x.index() as i32)
                .unwrap_or(-1);
            let normal_texture = entry
                .normal_texture_key
                .map(|x| x.index() as i32)
                .unwrap_or(-1);
            let emissive_texture = entry
                .emissive_texture_key
                .map(|x| x.index() as i32)
                .unwrap_or(-1);

            *material = mesh_adv_textured_frag::MaterialDbEntryBuffer {
                base_color_factor: material_data.base_color_factor,
                emissive_factor: material_data.emissive_factor,
                metallic_factor: material_data.metallic_factor,
                roughness_factor: material_data.roughness_factor,
                normal_texture_scale: material_data.normal_texture_scale,
                alpha_threshold: material_data.alpha_threshold,
                enable_alpha_blend: (material_data.blend_method == MeshAdvBlendMethod::AlphaBlend)
                    as u32,
                enable_alpha_clip: (material_data.blend_method == MeshAdvBlendMethod::AlphaClip)
                    as u32,
                color_texture,
                base_color_texture_has_alpha_channel: material_data
                    .base_color_texture_has_alpha_channel
                    as u32,
                metallic_roughness_texture,
                normal_texture,
                emissive_texture,
                _padding0: Default::default(),
            };
        }
        material_data_sbo.unmap_buffer()?;

        Ok(resource_allocator.insert_buffer(material_data_sbo))
    }

    pub fn update(&mut self) {
        for key in self.drop_rx.try_iter() {
            Self::do_remove_material(
                key,
                &mut self.material_entries,
                &mut self.material_data,
                &mut self.image_views,
                &mut self.image_view_lookup,
            );
        }
    }

    pub fn destroy(&mut self) {
        self.material_entries.clear();
        self.material_data.clear();
        self.image_views.clear();
        self.image_view_lookup.clear();
    }

    pub fn update_gpu_resources(
        &self,
        resource_context: &ResourceContext,
        bindless_materials_layout: &ResourceArc<DescriptorSetLayoutResource>,
        invalid_image: &ResourceArc<ImageViewResource>,
    ) -> RafxResult<DescriptorSetArc> {
        let all_materials = self.create_all_materials_buffer(resource_context)?;

        // Create array of textures
        let mut descriptor_set_allocator = resource_context.create_descriptor_set_allocator();
        let mut descriptor_set = descriptor_set_allocator
            .create_dyn_descriptor_set_uninitialized(bindless_materials_layout)?;

        descriptor_set.set_buffer(
            mesh_adv_textured_frag::ALL_MATERIALS_DESCRIPTOR_BINDING_INDEX as u32,
            &all_materials,
        );

        // For now this is necessary because 1) vulkan wants everything bound, unless opting into
        // certain features 2) there is nothing clearing old bindings from previous frames. We can
        // end up with textures from previous frames that were dropped being bound. (Even if we
        // don't try to index them, binding the descriptor set with stale resources can cause UB)
        for i in 0..768 {
            descriptor_set.set_image_at_index(
                mesh_adv_textured_frag::ALL_MATERIAL_TEXTURES_DESCRIPTOR_BINDING_INDEX as u32,
                i,
                invalid_image,
            );
        }

        //println!("set image");
        for (key, image_view) in self.image_views.iter() {
            descriptor_set.set_image_at_index(
                mesh_adv_textured_frag::ALL_MATERIAL_TEXTURES_DESCRIPTOR_BINDING_INDEX as u32,
                key.index() as usize,
                &image_view.image,
            );
        }
        //println!("finished set image");

        descriptor_set.flush(&mut descriptor_set_allocator)?;
        descriptor_set_allocator.flush_changes()?;

        // material data binding set?

        Ok(descriptor_set.descriptor_set().clone())
    }
}

type MeshAdvMaterialUploadOp = UploadAssetOp<MeshAdvMaterial, MeshMaterialAdvAsset>;

pub struct PendingMaterialUpload {
    upload_op: MeshAdvMaterialUploadOp,
    material_def: MeshAdvMaterialDef,
}

pub struct MaterialDBUploadQueue {
    pending_upload_tx: Sender<PendingMaterialUpload>,
    pending_upload_rx: Receiver<PendingMaterialUpload>,
}

impl MaterialDBUploadQueue {
    pub fn new() -> Self {
        let (pending_upload_tx, pending_upload_rx) = crossbeam_channel::unbounded();
        MaterialDBUploadQueue {
            pending_upload_tx,
            pending_upload_rx,
        }
    }

    pub fn material_upload_queue_context(&self) -> MaterialDBUploadQueueContext {
        MaterialDBUploadQueueContext {
            pending_upload_tx: self.pending_upload_tx.clone(),
        }
    }

    pub fn update(
        &self,
        material_db: &mut MaterialDB,
    ) {
        //println!("MaterialDBUploadQueue update");
        for pending_upload in self.pending_upload_rx.try_iter() {
            //log::info!("Processing pending upload");
            let result = material_db.add_material(pending_upload.material_def);
            pending_upload.upload_op.complete(result);
        }
    }
}

pub struct MaterialDBUploadQueueContext {
    pending_upload_tx: Sender<PendingMaterialUpload>,
}

impl MaterialDBUploadQueueContext {
    pub fn add_material(
        &self,
        upload_op: MeshAdvMaterialUploadOp,
        material_def: MeshAdvMaterialDef,
    ) -> RafxResult<()> {
        self.pending_upload_tx
            .send(PendingMaterialUpload {
                upload_op,
                material_def,
            })
            .map_err(|_err| {
                let error = format!("Could not enqueue material upload");
                log::error!("{}", error);
                RafxError::StringError(error)
            })
    }
}
