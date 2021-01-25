use crate::metal::{RafxDeviceContextMetal, RafxSamplerMetal};
use crate::{
    RafxDescriptorIndex, RafxPipelineType, RafxResourceType, RafxResult, RafxRootSignatureDef,
    MAX_DESCRIPTOR_SET_LAYOUTS,
};
use cocoa_foundation::foundation::NSUInteger;
use fnv::FnvHashMap;
use metal_rs::{MTLResourceUsage, MTLTextureType};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct ImmutableSampler {
    //pub(crate) binding: u32,
    pub(crate) samplers: Vec<RafxSamplerMetal>,

    pub(crate) argument_buffer_id: NSUInteger,
}

//TODO: Could compact this down quite a bit
#[derive(Clone, Debug)]
pub(crate) struct DescriptorInfo {
    pub(crate) name: Option<String>,
    pub(crate) resource_type: RafxResourceType,
    // Only valid for textures
    //pub(crate) texture_dimensions: Option<RafxTextureDimension>,

    // Also the set layout
    pub(crate) set_index: u32,
    // Binding within the set
    pub(crate) binding: u32,
    // Used for arrays of textures, samplers, etc.
    pub(crate) element_count: u32,
    // Index into DescriptorSetLayoutInfo::descriptors list
    // NOT THE BINDING INDEX!!!
    pub(crate) descriptor_index: RafxDescriptorIndex,

    // --- metal-specific ---
    //pub(crate) immutable_sampler: Option<Vec<RafxSampler>>,
    //pub(crate) usage: metal::MTLResourceUsage,
    pub(crate) argument_buffer_id: NSUInteger,
}

#[derive(Default, Debug)]
pub(crate) struct DescriptorSetLayoutInfo {
    // Settable descriptors, immutable samplers are omitted
    pub(crate) descriptors: Vec<RafxDescriptorIndex>,
    // Indexes binding index to the descriptors list
    pub(crate) binding_to_descriptor_index: FnvHashMap<u32, RafxDescriptorIndex>,

    // --- metal-specific ---
    // Now embedded by spirv_cross in the shader
    //pub(crate) immutable_samplers: Vec<ImmutableSampler>,
    // All argument buffer IDs must be within 0..argument_buffer_id_range
    pub(crate) argument_buffer_id_range: u32,
    // pub(crate) sampler_count: u32,
    // pub(crate) texture_count: u32,
    // pub(crate) buffer_count: u32,
}

#[derive(Debug)]
pub(crate) struct RafxRootSignatureMetalInner {
    pub(crate) device_context: RafxDeviceContextMetal,
    pub(crate) pipeline_type: RafxPipelineType,
    pub(crate) layouts: [DescriptorSetLayoutInfo; MAX_DESCRIPTOR_SET_LAYOUTS],
    pub(crate) descriptors: Vec<DescriptorInfo>,
    pub(crate) name_to_descriptor_index: FnvHashMap<String, RafxDescriptorIndex>,

    // --- metal-specific ---
    // Keeps them in scope so they don't drop
    //TODO: Can potentially remove, they are held in DescriptorInfo too
    //immutable_samplers: Vec<RafxSampler>,
    pub(crate) argument_descriptors:
        [Vec<metal_rs::ArgumentDescriptor>; MAX_DESCRIPTOR_SET_LAYOUTS],
    pub(crate) argument_buffer_resource_usages:
        [Arc<Vec<MTLResourceUsage>>; MAX_DESCRIPTOR_SET_LAYOUTS],
}

// for metal_rs::ArgumentDescriptor
unsafe impl Send for RafxRootSignatureMetalInner {}
unsafe impl Sync for RafxRootSignatureMetalInner {}

#[derive(Clone, Debug)]
pub struct RafxRootSignatureMetal {
    pub(crate) inner: Arc<RafxRootSignatureMetalInner>,
}

impl RafxRootSignatureMetal {
    pub fn device_context(&self) -> &RafxDeviceContextMetal {
        &self.inner.device_context
    }

    pub fn pipeline_type(&self) -> RafxPipelineType {
        self.inner.pipeline_type
    }

    pub fn find_descriptor_by_name(
        &self,
        name: &str,
    ) -> Option<RafxDescriptorIndex> {
        self.inner.name_to_descriptor_index.get(name).copied()
    }

    pub fn find_descriptor_by_binding(
        &self,
        set_index: u32,
        binding: u32,
    ) -> Option<RafxDescriptorIndex> {
        self.inner
            .layouts
            .get(set_index as usize)
            .and_then(|x| x.binding_to_descriptor_index.get(&binding))
            .copied()
    }

    pub(crate) fn descriptor(
        &self,
        descriptor_index: RafxDescriptorIndex,
    ) -> Option<&DescriptorInfo> {
        self.inner.descriptors.get(descriptor_index.0 as usize)
    }

    pub fn new(
        device_context: &RafxDeviceContextMetal,
        root_signature_def: &RafxRootSignatureDef,
    ) -> RafxResult<Self> {
        log::trace!("Create RafxRootSignatureMetal");

        // If we update this constant, update the arrays in this function
        assert_eq!(MAX_DESCRIPTOR_SET_LAYOUTS, 4);

        // let mut immutable_samplers = vec![];
        // for sampler_list in root_signature_def.immutable_samplers {
        //     for sampler in sampler_list.samplers {
        //         immutable_samplers.push(sampler.clone());
        //     }
        // }

        // Make sure all shaders are compatible/build lookup of shared data from them
        let (pipeline_type, mut merged_resources, _merged_resources_name_index_map) =
            crate::internal_shared::merge_resources(root_signature_def)?;

        merged_resources.sort_by(|lhs, rhs| lhs.binding.cmp(&rhs.binding));

        let mut layouts = [
            DescriptorSetLayoutInfo::default(),
            DescriptorSetLayoutInfo::default(),
            DescriptorSetLayoutInfo::default(),
            DescriptorSetLayoutInfo::default(),
        ];

        let mut resource_usages = [vec![], vec![], vec![], vec![]];

        let mut next_argument_buffer_id = [0, 0, 0, 0];

        let mut descriptors = Vec::with_capacity(merged_resources.len());
        let mut name_to_descriptor_index = FnvHashMap::default();

        for resource in &merged_resources {
            resource.validate()?;

            // Not currently supported
            assert_ne!(resource.resource_type, RafxResourceType::ROOT_CONSTANT);

            // Verify set index is valid

            let immutable_sampler = crate::internal_shared::find_immutable_sampler_index(
                root_signature_def.immutable_samplers,
                &resource.name,
                resource.set_index,
                resource.binding,
            );

            // Check that if an immutable sampler is set, the array size matches the resource element count
            if let Some(immutable_sampler_index) = immutable_sampler {
                if resource.element_count_normalized() as usize
                    != root_signature_def.immutable_samplers[immutable_sampler_index]
                        .samplers
                        .len()
                {
                    Err(format!(
                        "Descriptor (set={:?} binding={:?}) named {:?} specifies {} elements but the count of provided immutable samplers ({}) did not match",
                        resource.set_index,
                        resource.binding,
                        resource.name,
                        resource.element_count_normalized(),
                        root_signature_def.immutable_samplers[immutable_sampler_index].samplers.len()
                    ))?;
                }
            }

            let layout: &mut DescriptorSetLayoutInfo = &mut layouts[resource.set_index as usize];

            let descriptor_index = RafxDescriptorIndex(descriptors.len() as u32);

            let argument_buffer_id = next_argument_buffer_id[resource.set_index as usize];
            next_argument_buffer_id[resource.set_index as usize] +=
                resource.element_count_normalized();

            //let update_data_offset_in_set = Some(layout.update_data_count_per_set);

            if let Some(_immutable_sampler_index) = immutable_sampler {
                // This is now embedded by spirv_cross in the shader
                // let samplers = root_signature_def
                //     .immutable_samplers[immutable_sampler_index]
                //     .samplers
                //     .iter()
                //     .map(|x| x.metal_sampler().unwrap().clone())
                //     .collect();
                //
                // layout.immutable_samplers.push(ImmutableSampler {
                //     //binding: resource.binding,
                //     samplers,
                //     argument_buffer_id: argument_buffer_id as _
                // });
            } else {
                // Add it to the descriptor list
                descriptors.push(DescriptorInfo {
                    name: resource.name.clone(),
                    resource_type: resource.resource_type,
                    //texture_dimensions: resource.texture_dimensions,
                    set_index: resource.set_index,
                    binding: resource.binding,
                    element_count: resource.element_count_normalized(),
                    descriptor_index,
                    //immutable_sampler: immutable_sampler.map(|x| immutable_samplers[x].clone()),
                    //update_data_offset_in_set,
                    //usage
                    argument_buffer_id: argument_buffer_id as _,
                });

                if let Some(name) = resource.name.as_ref() {
                    name_to_descriptor_index.insert(name.clone(), descriptor_index);
                }

                layout.descriptors.push(descriptor_index);
                layout
                    .binding_to_descriptor_index
                    .insert(resource.binding, descriptor_index);
                layout.argument_buffer_id_range =
                    next_argument_buffer_id[resource.set_index as usize];

                // Build out the MTLResourceUsage usages - it's used when we bind descriptor sets
                let layout_resource_usages = &mut resource_usages[resource.set_index as usize];
                layout_resource_usages.resize(
                    layout.argument_buffer_id_range as usize,
                    MTLResourceUsage::empty(),
                );
                let usage = super::util::resource_type_mtl_resource_usage(resource.resource_type);
                for i in argument_buffer_id..layout.argument_buffer_id_range {
                    layout_resource_usages[i as usize] = usage;
                }

                debug_assert_ne!(layout.argument_buffer_id_range, 0);
            }
        }

        let mut argument_descriptors = [vec![], vec![], vec![], vec![]];

        for i in 0..MAX_DESCRIPTOR_SET_LAYOUTS {
            for &resource_index in &layouts[i].descriptors {
                let descriptor = &descriptors[resource_index.0 as usize];

                let argument_descriptor = metal_rs::ArgumentDescriptor::new();

                let access =
                    super::util::resource_type_mtl_argument_access(descriptor.resource_type);
                let data_type =
                    super::util::resource_type_mtl_data_type(descriptor.resource_type).unwrap();
                argument_descriptor.set_access(access);
                argument_descriptor.set_array_length(descriptor.element_count as _);
                argument_descriptor.set_data_type(data_type);
                argument_descriptor.set_index(descriptor.argument_buffer_id as _);
                argument_descriptor.set_texture_type(MTLTextureType::D2); //TODO: Temp, not sure if it's this gets changed when bound
                argument_descriptors[i].push(argument_descriptor.to_owned());
            }
        }

        let argument_buffer_resource_usages = [
            Arc::new(std::mem::take(&mut resource_usages[0])),
            Arc::new(std::mem::take(&mut resource_usages[1])),
            Arc::new(std::mem::take(&mut resource_usages[2])),
            Arc::new(std::mem::take(&mut resource_usages[3])),
        ];

        let inner = RafxRootSignatureMetalInner {
            device_context: device_context.clone(),
            pipeline_type,
            layouts,
            descriptors,
            name_to_descriptor_index,
            argument_buffer_resource_usages,
            argument_descriptors,
        };

        Ok(RafxRootSignatureMetal {
            inner: Arc::new(inner),
        })
    }
}
