use ash::vk;
use renderer_assets::pipeline_description as dsc;
use crate::resource_managers::resource_lookup::{ImageViewResource, ResourceLookupSet};
use fnv::FnvHashMap;
use crate::resource_managers::asset_lookup::{LoadedMaterialPass, LoadedAssetLookupSet, SlotNameLookup};
use renderer_assets::pipeline::pipeline::MaterialInstanceSlotAssignment;
use ash::prelude::VkResult;
use atelier_assets::loader::handle::AssetHandle;
use crate::resource_managers::descriptor_sets::DescriptorSetWriteBuffer;
use crate::resource_managers::ResourceArc;

//
// These represent descriptor updates that can be applied to a descriptor set in a pool
//
#[derive(Debug, Clone)]
pub enum DescriptorSetWriteElementImageValue {
    Raw(vk::ImageView),
    Resource(ResourceArc<ImageViewResource>),
}

impl DescriptorSetWriteElementImageValue {
    pub fn get_raw(&self) -> vk::ImageView {
        match self {
            DescriptorSetWriteElementImageValue::Raw(view) => *view,
            DescriptorSetWriteElementImageValue::Resource(resource) => resource.get_raw().image_view
        }
    }
}

// The information needed to write image metadata for a descriptor
#[derive(Debug, Clone, Default)]
pub struct DescriptorSetWriteElementImage {
    pub sampler: Option<ResourceArc<vk::Sampler>>,
    pub image_view: Option<DescriptorSetWriteElementImageValue>,
    // For now going to assume layout is always ShaderReadOnlyOptimal
    //pub image_info: vk::DescriptorImageInfo,
}

// Info needed to write a buffer reference to a descriptor set
#[derive(Debug, Clone)]
pub struct DescriptorSetWriteElementBufferDataBufferRef {
    pub buffer: ResourceArc<vk::Buffer>,
    pub offset: vk::DeviceSize,
    pub size: vk::DeviceSize, // may use vk::WHOLE_SIZE
}

#[derive(Debug, Clone)]
pub enum DescriptorSetWriteElementBufferData {
    BufferRef(DescriptorSetWriteElementBufferDataBufferRef),
    Data(Vec<u8>)
}

// The information needed to write buffer metadata for a descriptor
#[derive(Debug, Clone, Default)]
pub struct DescriptorSetWriteElementBuffer {
    pub buffer: Option<DescriptorSetWriteElementBufferData>
}

// All the data required to overwrite a descriptor. The image/buffer infos will be populated depending
// on the descriptor's type
#[derive(Debug, Clone)]
pub struct DescriptorSetElementWrite {
    // This is a complete spec for
    pub descriptor_type: dsc::DescriptorType,

    //TODO: Should these be Option<Vec>?
    pub image_info: Vec<DescriptorSetWriteElementImage>,
    pub buffer_info: Vec<DescriptorSetWriteElementBuffer>,
    //TODO: texel buffer view support
    //pub p_texel_buffer_view: *const BufferView,

    // If true, we are not permitted to modify samplers via the write. It's a bit of a hack having
    // this here since we are using this struct both to define a write and to store the metadata
    // for an already-written descriptor. The issue is that I'd like runtime checking that we don't
    // try to rebind a sampler and the easiest way to track this metadata is to include it here.
    // Potentially we could have a separate type that contains the other values plus this bool.
    pub has_immutable_sampler: bool,
}

// Represents an "index" into a single binding within a layout. A binding can be in the form of an
// array, but for now this is not supported
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DescriptorSetElementKey {
    pub dst_binding: u32,
    //pub dst_array_element: u32,
}

// A set of writes to descriptors within a descriptor set
#[derive(Debug, Default, Clone)]
pub struct DescriptorSetWriteSet {
    pub elements: FnvHashMap<DescriptorSetElementKey, DescriptorSetElementWrite>,
}

impl DescriptorSetWriteSet {
    pub fn copy_from(&mut self, other: &DescriptorSetWriteSet) {
        for (k, v) in other.elements.iter() {
            self.elements.insert(k.clone(), v.clone());
        }
    }
}

pub fn create_uninitialized_write_set_for_layout(
    layout: &dsc::DescriptorSetLayout
) -> DescriptorSetWriteSet {
    let mut write_set = DescriptorSetWriteSet::default();
    for (binding_index, binding) in layout.descriptor_set_layout_bindings.iter().enumerate() {
        let key = DescriptorSetElementKey {
            dst_binding: binding_index as u32,
            //dst_array_element: 0,
        };

        let mut element_write = DescriptorSetElementWrite {
            has_immutable_sampler: binding.immutable_samplers.is_some(),
            descriptor_type: binding.descriptor_type.into(),
            image_info: Default::default(),
            buffer_info: Default::default(),
        };

        let what_to_bind = super::what_to_bind(&element_write);

        if what_to_bind.bind_images || what_to_bind.bind_samplers {
            element_write.image_info.resize(
                binding.descriptor_count as usize,
                DescriptorSetWriteElementImage::default(),
            );
        }

        if what_to_bind.bind_buffers {
            element_write.buffer_info.resize(
                binding.descriptor_count as usize,
                DescriptorSetWriteElementBuffer::default(),
            );
        }

        write_set.elements.insert(key, element_write);
    }

    write_set
}

pub fn apply_material_instance_slot_assignment(
    slot_assignment: &MaterialInstanceSlotAssignment,
    pass_slot_name_lookup: &SlotNameLookup,
    assets: &LoadedAssetLookupSet,
    resources: &mut ResourceLookupSet,
    material_pass_write_set: &mut Vec<DescriptorSetWriteSet>,
) -> VkResult<()> {
    if let Some(slot_locations) = pass_slot_name_lookup.get(&slot_assignment.slot_name) {
        for location in slot_locations {
            let mut layout_descriptor_set_writes =
                &mut material_pass_write_set[location.layout_index as usize];
            let write = layout_descriptor_set_writes
                .elements
                .get_mut(&DescriptorSetElementKey {
                    dst_binding: location.binding_index,
                    //dst_array_element: location.array_index
                })
                .unwrap();

            let what_to_bind = super::what_to_bind(write);

            if what_to_bind.bind_images || what_to_bind.bind_samplers {
                let mut write_image = DescriptorSetWriteElementImage {
                    image_view: None,
                    sampler: None,
                };

                if what_to_bind.bind_images {
                    if let Some(image) = &slot_assignment.image {
                        let loaded_image = assets.images.get_latest(image.load_handle()).unwrap();
                        write_image.image_view = Some(DescriptorSetWriteElementImageValue::Resource(loaded_image.image_view.clone()));
                    }
                }

                if what_to_bind.bind_samplers {
                    if let Some(sampler) = &slot_assignment.sampler {
                        let sampler = resources.get_or_create_sampler(sampler)?;
                        write_image.sampler = Some(sampler);
                    }
                }

                write.image_info = vec![write_image];
            }

            if what_to_bind.bind_buffers {
                let mut write_buffer = DescriptorSetWriteElementBuffer {
                    buffer: None
                };

                if let Some(buffer_data) = &slot_assignment.buffer_data {
                    write_buffer.buffer = Some(DescriptorSetWriteElementBufferData::Data(buffer_data.clone()));
                }

                write.buffer_info = vec![write_buffer];
            }
        }
    }

    Ok(())
}

pub fn create_uninitialized_write_sets_for_material_pass(
    pass: &LoadedMaterialPass
) -> Vec<DescriptorSetWriteSet> {
    // The metadata for the descriptor sets within this pass, one for each set within the pass
    let descriptor_set_layouts = &pass.shader_interface.descriptor_set_layouts;

    let mut pass_descriptor_set_writes: Vec<_> = descriptor_set_layouts
        .iter()
        .map(|layout| create_uninitialized_write_set_for_layout(&layout.into()))
        .collect();

    pass_descriptor_set_writes
}

pub fn create_write_sets_for_material_instance_pass(
    pass: &LoadedMaterialPass,
    slots: &Vec<MaterialInstanceSlotAssignment>,
    assets: &LoadedAssetLookupSet,
    resources: &mut ResourceLookupSet,
) -> VkResult<Vec<DescriptorSetWriteSet>> {
    let mut pass_descriptor_set_writes = create_uninitialized_write_sets_for_material_pass(pass);

    //
    // Now modify the descriptor set writes to actually point at the things specified by the material
    //
    for slot in slots {
        apply_material_instance_slot_assignment(
            slot,
            &pass.pass_slot_name_lookup,
            assets,
            resources,
            &mut pass_descriptor_set_writes,
        )?;
    }

    Ok(pass_descriptor_set_writes)
}
