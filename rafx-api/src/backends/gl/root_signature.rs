use crate::gl::reflection::{UniformIndex, UniformReflectionData};
use crate::gl::{ProgramId, RafxDeviceContextGles2};
use crate::{
    RafxDescriptorIndex, RafxPipelineType, RafxResourceType, RafxResult, RafxRootSignatureDef,
    MAX_DESCRIPTOR_SET_LAYOUTS,
};
use fnv::FnvHashMap;
use std::ffi::CString;
use std::sync::atomic::Ordering;
use std::sync::Arc;

static NEXT_ROOT_SIGNATURE_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

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

    // Indexed by the index of the shader passed into the root descriptor. It may not be the same
    // for different shaders.
    pub(crate) uniform_index: Option<UniformIndex>,

    // descriptor sets contain a list of textures, buffers, etc. This indicates where in the list
    // this descriptor starts.
    pub(crate) descriptor_data_offset_in_set: Option<u32>,

    pub(crate) gl_name: CString,
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
    // // pub(crate) sampler_count: u32,
    pub(crate) image_descriptor_state_count: u32,
    pub(crate) buffer_descriptor_state_count: u32,
}

#[derive(Debug)]
pub(crate) struct RafxRootSignatureGles2Inner {
    pub(crate) device_context: RafxDeviceContextGles2,
    pub(crate) pipeline_type: RafxPipelineType,
    pub(crate) layouts: [DescriptorSetLayoutInfo; MAX_DESCRIPTOR_SET_LAYOUTS],
    pub(crate) descriptors: Vec<DescriptorInfo>,
    pub(crate) name_to_descriptor_index: FnvHashMap<String, RafxDescriptorIndex>,
    //
    // // --- gl-specific ---
    // // Keeps them in scope so they don't drop
    // //TODO: Can potentially remove, they are held in DescriptorInfo too
    // //immutable_samplers: Vec<RafxSampler>,
    pub(crate) uniform_reflection: UniformReflectionData,
    pub(crate) root_signature_id: u32,
}

#[derive(Clone, Debug)]
pub struct RafxRootSignatureGles2 {
    pub(crate) inner: Arc<RafxRootSignatureGles2Inner>,
}

impl PartialEq for RafxRootSignatureGles2 {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.inner.root_signature_id == other.inner.root_signature_id
    }
}

impl RafxRootSignatureGles2 {
    pub fn device_context(&self) -> &RafxDeviceContextGles2 {
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

    pub(crate) fn uniform_reflection_data(&self) -> &UniformReflectionData {
        &self.inner.uniform_reflection
    }

    pub(crate) fn uniform_index(
        &self,
        descriptor_index: RafxDescriptorIndex,
    ) -> Option<UniformIndex> {
        self.inner.descriptors[descriptor_index.0 as usize].uniform_index
    }

    pub fn new(
        device_context: &RafxDeviceContextGles2,
        root_signature_def: &RafxRootSignatureDef,
    ) -> RafxResult<Self> {
        log::trace!("Create RafxRootSignatureGl");

        //TODO: Count textures and verify we don't exceed max supported

        // If we update this constant, update the arrays in this function
        assert_eq!(MAX_DESCRIPTOR_SET_LAYOUTS, 4);

        if !root_signature_def.immutable_samplers.is_empty() {
            unimplemented!();
        }

        // let mut immutable_samplers = vec![];
        // for sampler_list in root_signature_def.immutable_samplers {
        //     for sampler in sampler_list.samplers {
        //         immutable_samplers.push(sampler.clone());
        //     }
        // }

        let gl_context = device_context.gl_context();

        // Make sure all shaders are compatible/build lookup of shared data from them
        let (pipeline_type, merged_resources, _merged_resources_name_index_map) =
            crate::internal_shared::merge_resources(root_signature_def)?;

        let mut layouts = [
            DescriptorSetLayoutInfo::default(),
            DescriptorSetLayoutInfo::default(),
            DescriptorSetLayoutInfo::default(),
            DescriptorSetLayoutInfo::default(),
        ];

        // These are used to populate descriptor_data_offset_in_set in a descriptor, which is later used
        // to index into Vecs in DescriptorSetArrayData
        let mut next_descriptor_data_buffer_offset = [0, 0, 0, 0];
        let mut next_descriptor_data_image_offset = [0, 0, 0, 0];

        let mut descriptors = Vec::with_capacity(merged_resources.len());
        let mut name_to_descriptor_index = FnvHashMap::default();

        let program_ids: Vec<ProgramId> = root_signature_def
            .shaders
            .iter()
            .map(|x| x.gles2_shader().unwrap().gl_program_id())
            .collect();

        // Lookup for uniform fields
        let uniform_reflection =
            UniformReflectionData::new(gl_context, &program_ids, root_signature_def.shaders)?;

        for resource in &merged_resources {
            resource.validate()?;

            let descriptor_data_offset_in_set;
            if resource.resource_type.intersects(
                RafxResourceType::TEXTURE
                    | RafxResourceType::TEXTURE_READ_WRITE
                    | RafxResourceType::SAMPLER,
            ) {
                descriptor_data_offset_in_set =
                    Some(next_descriptor_data_image_offset[resource.set_index as usize]);
                next_descriptor_data_image_offset[resource.set_index as usize] +=
                    resource.element_count_normalized();
            } else if resource.resource_type.intersects(
                RafxResourceType::BUFFER
                    | RafxResourceType::BUFFER_READ_WRITE
                    | RafxResourceType::UNIFORM_BUFFER,
            ) {
                descriptor_data_offset_in_set =
                    Some(next_descriptor_data_buffer_offset[resource.set_index as usize]);
                next_descriptor_data_buffer_offset[resource.set_index as usize] +=
                    resource.element_count_normalized();
            } else {
                return Err(format!(
                    "Resource type {:?} not supporrted by GL ES",
                    resource.resource_type
                ))?;
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
                let descriptor_index = RafxDescriptorIndex(descriptors.len() as u32);

                let gl_name = resource.gl_name.as_ref().unwrap();
                let gl_name_cstr = CString::new(gl_name.as_str()).unwrap();

                let uniform_index = if resource.resource_type == RafxResourceType::UNIFORM_BUFFER {
                    // May be none if the variable is not active in any shader
                    uniform_reflection.uniform_index(gl_name)
                } else {
                    None
                };

                // Add it to the descriptor list
                descriptors.push(DescriptorInfo {
                    name: resource.name.clone(),
                    resource_type: resource.resource_type,
                    //texture_dimensions: resource.texture_dimensions,
                    set_index: resource.set_index,
                    binding: resource.binding,
                    element_count: resource.element_count_normalized(),
                    descriptor_index,
                    // immutable_sampler: immutable_sampler.map(|x| immutable_samplers[x].clone()),
                    uniform_index,
                    descriptor_data_offset_in_set,
                    gl_name: gl_name_cstr,
                });

                if let Some(name) = resource.name.as_ref() {
                    name_to_descriptor_index.insert(name.clone(), descriptor_index);
                }

                layout.descriptors.push(descriptor_index);
                layout
                    .binding_to_descriptor_index
                    .insert(resource.binding, descriptor_index);
            }
        }

        for i in 0..MAX_DESCRIPTOR_SET_LAYOUTS {
            layouts[i].image_descriptor_state_count = next_descriptor_data_image_offset[i];
            layouts[i].buffer_descriptor_state_count = next_descriptor_data_buffer_offset[i];
        }

        let root_signature_id = NEXT_ROOT_SIGNATURE_ID.fetch_add(1, Ordering::Relaxed);

        let inner = RafxRootSignatureGles2Inner {
            device_context: device_context.clone(),
            pipeline_type,
            layouts,
            descriptors,
            name_to_descriptor_index,
            uniform_reflection,
            root_signature_id,
        };

        Ok(RafxRootSignatureGles2 {
            inner: Arc::new(inner),
        })
    }
}
