use crate::resources::resource_lookup::{ImageViewResource, SamplerResource};
use crate::resources::ResourceArc;
use crate::BufferResource;
use fnv::FnvHashMap;
use rafx_api::RafxTexture;
use rafx_api::{RafxReflectedDescriptorSetLayout, RafxResourceType};

//
// These represent descriptor updates that can be applied to a descriptor set in a pool
//
#[derive(Debug, Clone)]
pub enum DescriptorSetWriteElementImageValue {
    Resource(ResourceArc<ImageViewResource>),
}

impl DescriptorSetWriteElementImageValue {
    pub fn get_image(&self) -> RafxTexture {
        match self {
            DescriptorSetWriteElementImageValue::Resource(resource) => {
                resource.get_raw().image.get_raw().image.clone()
            }
        }
    }
}

// The information needed to write image metadata for a descriptor
#[derive(Debug, Clone, Default)]
pub struct DescriptorSetWriteElementImage {
    pub sampler: Option<ResourceArc<SamplerResource>>,
    pub image_view: Option<DescriptorSetWriteElementImageValue>,
}

// Info needed to write a buffer reference to a descriptor set
#[derive(Debug, Clone)]
pub struct DescriptorSetWriteElementBufferDataBufferRef {
    pub buffer: ResourceArc<BufferResource>,
    pub byte_offset: Option<u64>,
    pub size: Option<u64>,
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
    pub descriptor_type: RafxResourceType,

    pub image_info: DescriptorSetWriteElementImage,
    pub buffer_info: DescriptorSetWriteElementBuffer,

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
    pub array_index: usize,
}

// Represents an "index" of a binding within a layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DescriptorSetBindingKey {
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
    layout: &RafxReflectedDescriptorSetLayout
) -> DescriptorSetWriteSet {
    let mut write_set = DescriptorSetWriteSet::default();
    for binding in &layout.bindings {
        for array_index in 0..binding.resource.element_count_normalized() {
            let element_write = DescriptorSetElementWrite {
                has_immutable_sampler: binding.immutable_samplers.is_some(),
                descriptor_type: binding.resource.resource_type,
                image_info: DescriptorSetWriteElementImage::default(),
                buffer_info: DescriptorSetWriteElementBuffer::default(),
            };

            let key = DescriptorSetElementKey {
                dst_binding: binding.resource.binding as u32,
                array_index: array_index as usize,
            };

            write_set.elements.insert(key, element_write);
        }
    }

    write_set
}
