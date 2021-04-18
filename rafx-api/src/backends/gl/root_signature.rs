use crate::gl::{RafxDeviceContextGl, RafxSamplerGl, ProgramId};
use crate::{
    RafxDescriptorIndex, RafxPipelineType, RafxResourceType, RafxResult, RafxRootSignatureDef,
    MAX_DESCRIPTOR_SET_LAYOUTS,
};
//use cocoa_foundation::foundation::NSUInteger;
use fnv::FnvHashMap;
//use gl_rs::{MTLResourceUsage, MTLTextureType};
use std::sync::Arc;
use crate::gl::reflection::{UniformReflectionData, UniformIndex};

// #[derive(Debug)]
// pub(crate) struct ImmutableSampler {
//     //pub(crate) binding: u32,
//     pub(crate) samplers: Vec<RafxSamplerGl>,
//
//     pub(crate) argument_buffer_id: NSUInteger,
// }
//
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

    // // --- gl-specific ---
    // //pub(crate) immutable_sampler: Option<Vec<RafxSampler>>,
    // //pub(crate) usage: gl::MTLResourceUsage,
    // pub(crate) argument_buffer_id: NSUInteger,

    // Indexed by the index of the shader passed into the root descriptor. It may not be the same
    // for different shaders.
    //pub(crate) gl_locations: Vec<u32>,
    //size?
    //type?
    pub(crate) uniform_index: Option<UniformIndex>,

    // descriptor sets contain a list of textures, buffers, etc. This indicates where in the list
    // this descriptor starts.
    pub(crate) descriptor_data_offset_in_set: Option<u32>,
}

#[derive(Default, Debug)]
pub(crate) struct DescriptorSetLayoutInfo {
    // Settable descriptors, immutable samplers are omitted
    pub(crate) descriptors: Vec<RafxDescriptorIndex>,
    // Indexes binding index to the descriptors list
    pub(crate) binding_to_descriptor_index: FnvHashMap<u32, RafxDescriptorIndex>,

    // // --- gl-specific ---
    // // Now embedded by spirv_cross in the shader
    // //pub(crate) immutable_samplers: Vec<ImmutableSampler>,
    // All argument buffer IDs must be within 0..argument_buffer_id_range
    // //pub(crate) argument_buffer_id_range: u32,
    // // pub(crate) sampler_count: u32,
    // // pub(crate) texture_count: u32,
    // // pub(crate) buffer_count: u32,
}

#[derive(Debug)]
pub(crate) struct RafxRootSignatureGlInner {
    pub(crate) device_context: RafxDeviceContextGl,
    pub(crate) pipeline_type: RafxPipelineType,
    pub(crate) layouts: [DescriptorSetLayoutInfo; MAX_DESCRIPTOR_SET_LAYOUTS],
    pub(crate) descriptors: Vec<DescriptorInfo>,
    pub(crate) name_to_descriptor_index: FnvHashMap<String, RafxDescriptorIndex>,
    //
    // // --- gl-specific ---
    // // Keeps them in scope so they don't drop
    // //TODO: Can potentially remove, they are held in DescriptorInfo too
    // //immutable_samplers: Vec<RafxSampler>,
    // pub(crate) argument_descriptors:
    //     [Vec<gl_rs::ArgumentDescriptor>; MAX_DESCRIPTOR_SET_LAYOUTS],
    // pub(crate) argument_buffer_resource_usages:
    //     [Arc<Vec<MTLResourceUsage>>; MAX_DESCRIPTOR_SET_LAYOUTS],

    pub(crate) program_targets: Vec<ProgramId>,
    pub(crate) uniform_reflection: UniformReflectionData,
}

#[derive(Clone, Debug)]
pub struct RafxRootSignatureGl {
    pub(crate) inner: Arc<RafxRootSignatureGlInner>,
}

impl RafxRootSignatureGl {
    pub fn device_context(&self) -> &RafxDeviceContextGl {
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
        device_context: &RafxDeviceContextGl,
        root_signature_def: &RafxRootSignatureDef,
    ) -> RafxResult<Self> {
        // log::trace!("Create RafxRootSignatureGl");
        //
        // // Make sure all shaders are compatible/build lookup of shared data from them
        // let (pipeline_type, mut merged_resources, _merged_resources_name_index_map) =
        //     crate::internal_shared::merge_resources(root_signature_def)?;
        //
        // merged_resources.sort_by(|lhs, rhs| lhs.binding.cmp(&rhs.binding));
        //
        // //TODO: Count textures and verify we don't exceed max supported












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

        //let mut resource_usages = [vec![], vec![], vec![], vec![]];

        //let mut next_argument_buffer_id = [0, 0, 0, 0];

        // These are used to populate descriptor_data_offset_in_set in a descriptor, which is later used
        // to index into Vecs in DescriptorSetArrayData
        let mut next_descriptor_data_buffer_offset = [0, 0, 0, 0];
        let mut next_descriptor_data_image_offset = [0, 0, 0, 0];

        let mut descriptors = Vec::with_capacity(merged_resources.len());
        let mut name_to_descriptor_index = FnvHashMap::default();

        let program_ids : Vec<ProgramId> = root_signature_def.shaders.iter().map(|x| x.gl_shader().unwrap().gl_program_id()).collect();
        let uniform_reflection = UniformReflectionData::new(device_context.gl_context(), program_ids)?;
        println!("uniform reflection data:\n{:#?}", uniform_reflection);

        for resource in &merged_resources {
            resource.validate()?;

            let mut descriptor_data_offset_in_set = None;

            match resource.resource_type {
                RafxResourceType::TEXTURE | RafxResourceType::TEXTURE_READ_WRITE => {
                    descriptor_data_offset_in_set = Some(next_descriptor_data_image_offset[resource.set_index as usize]);
                    next_descriptor_data_image_offset[resource.set_index as usize] += resource.element_count;
                },
                RafxResourceType::BUFFER | RafxResourceType::BUFFER_READ_WRITE | RafxResourceType::UNIFORM_BUFFER => {
                    descriptor_data_offset_in_set = Some(next_descriptor_data_buffer_offset[resource.set_index as usize]);
                    next_descriptor_data_buffer_offset[resource.set_index as usize] += resource.element_count;
                },
            }

            //let mut gl_locations = vec![];
            for shader in root_signature_def.shaders {
                //let location = device_context.gl_context().gl_get_uniform_location(shader.gl_shader().unwrap().gl_program_id(), resource.name.as_ref().unwrap());
                //println!("location: {:?}", location);

                println!("shader {:?}", shader);
                //device_context.gl_context().build_uniform_reflection_data(shader.gl_shader().unwrap().gl_program_id());
            }

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

            // let argument_buffer_id = next_argument_buffer_id[resource.set_index as usize];
            // next_argument_buffer_id[resource.set_index as usize] +=
            //     resource.element_count_normalized();

            //let update_data_offset_in_set = Some(layout.update_data_count_per_set);

            if let Some(_immutable_sampler_index) = immutable_sampler {
                // This is now embedded by spirv_cross in the shader
                // let samplers = root_signature_def
                //     .immutable_samplers[immutable_sampler_index]
                //     .samplers
                //     .iter()
                //     .map(|x| x.gl_sampler().unwrap().clone())
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
                    //gl_locations
                    // // immutable_sampler: immutable_sampler.map(|x| immutable_samplers[x].clone()),
                    // // update_data_offset_in_set,
                    // // usage
                    // argument_buffer_id: argument_buffer_id as _,
                    uniform_index: None,
                    descriptor_data_offset_in_set: None,
                });

                if let Some(name) = resource.name.as_ref() {
                    name_to_descriptor_index.insert(name.clone(), descriptor_index);
                }

                layout.descriptors.push(descriptor_index);
                layout
                    .binding_to_descriptor_index
                    .insert(resource.binding, descriptor_index);
                // layout.argument_buffer_id_range =
                //     next_argument_buffer_id[resource.set_index as usize];

                // // Build out the MTLResourceUsage usages - it's used when we bind descriptor sets
                // let layout_resource_usages = &mut resource_usages[resource.set_index as usize];
                // layout_resource_usages.resize(
                //     layout.argument_buffer_id_range as usize,
                //     MTLResourceUsage::empty(),
                // );
                // let usage = super::util::resource_type_mtl_resource_usage(resource.resource_type);
                // for i in argument_buffer_id..layout.argument_buffer_id_range {
                //     layout_resource_usages[i as usize] = usage;
                // }

                //debug_assert_ne!(layout.argument_buffer_id_range, 0);
            }
        }

        // let mut argument_descriptors = [vec![], vec![], vec![], vec![]];

        // for i in 0..MAX_DESCRIPTOR_SET_LAYOUTS {
        //     for &resource_index in &layouts[i].descriptors {
        //         let descriptor = &descriptors[resource_index.0 as usize];
        //
        //         let argument_descriptor = gl_rs::ArgumentDescriptor::new();
        //
        //         let access =
        //             super::util::resource_type_mtl_argument_access(descriptor.resource_type);
        //         let data_type =
        //             super::util::resource_type_mtl_data_type(descriptor.resource_type).unwrap();
        //         argument_descriptor.set_access(access);
        //         argument_descriptor.set_array_length(descriptor.element_count as _);
        //         argument_descriptor.set_data_type(data_type);
        //         argument_descriptor.set_index(descriptor.argument_buffer_id as _);
        //         argument_descriptor.set_texture_type(MTLTextureType::D2); //TODO: Temp, not sure if it's this gets changed when bound
        //         argument_descriptors[i].push(argument_descriptor.to_owned());
        //     }
        // }
        //
        // let argument_buffer_resource_usages = [
        //     Arc::new(std::mem::take(&mut resource_usages[0])),
        //     Arc::new(std::mem::take(&mut resource_usages[1])),
        //     Arc::new(std::mem::take(&mut resource_usages[2])),
        //     Arc::new(std::mem::take(&mut resource_usages[3])),
        // ];

        let inner = RafxRootSignatureGlInner {
            device_context: device_context.clone(),
            pipeline_type,
            layouts,
            descriptors,
            name_to_descriptor_index,
            program_targets: vec![],
            //argument_buffer_resource_usages,
            //argument_descriptors,
            uniform_reflection,
        };

        Ok(RafxRootSignatureGl {
            inner: Arc::new(inner),
        })
    }
}
