use crate::resources::resource_lookup::{ImageViewResource, SamplerResource};
use crate::resources::ResourceArc;
use crate::{vk_description as dsc, BufferResource};
use ash::vk;
use fnv::FnvHashMap;

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
            DescriptorSetWriteElementImageValue::Resource(resource) => {
                resource.get_raw().image_view
            }
        }
    }
}

// The information needed to write image metadata for a descriptor
#[derive(Debug, Clone, Default)]
pub struct DescriptorSetWriteElementImage {
    pub sampler: Option<ResourceArc<SamplerResource>>,
    pub image_view: Option<DescriptorSetWriteElementImageValue>,
    // For now going to assume layout is always ShaderReadOnlyOptimal
    //pub image_info: vk::DescriptorImageInfo,
}

// Info needed to write a buffer reference to a descriptor set
#[derive(Debug, Clone)]
pub struct DescriptorSetWriteElementBufferDataBufferRef {
    pub buffer: ResourceArc<BufferResource>,
    pub offset: vk::DeviceSize,
    pub size: vk::DeviceSize, // may use vk::WHOLE_SIZE
}

#[derive(Debug, Clone)]
pub enum DescriptorSetWriteElementBufferData {
    BufferRef(DescriptorSetWriteElementBufferDataBufferRef),
    Data(Vec<u8>),
}

// The information needed to write buffer metadata for a descriptor
#[derive(Debug, Clone, Default)]
pub struct DescriptorSetWriteElementBuffer {
    pub buffer: Option<DescriptorSetWriteElementBufferData>,
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
}

// A set of writes to descriptors within a descriptor set
#[derive(Debug, Default, Clone)]
pub struct DescriptorSetWriteSet {
    pub elements: FnvHashMap<DescriptorSetElementKey, DescriptorSetElementWrite>,
}

impl DescriptorSetWriteSet {
    pub fn copy_from(
        &mut self,
        other: &DescriptorSetWriteSet,
    ) {
        for (k, v) in other.elements.iter() {
            self.elements.insert(*k, v.clone());
        }
    }
}

pub fn create_uninitialized_write_set_for_layout(
    layout: &dsc::DescriptorSetLayout
) -> DescriptorSetWriteSet {
    let mut write_set = DescriptorSetWriteSet::default();
    for binding in &layout.descriptor_set_layout_bindings {
        let key = DescriptorSetElementKey {
            dst_binding: binding.binding as u32,
        };

        let mut element_write = DescriptorSetElementWrite {
            has_immutable_sampler: binding.immutable_samplers.is_some(),
            descriptor_type: binding.descriptor_type,
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
