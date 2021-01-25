use crate::vulkan::RafxDeviceContextVulkan;
use crate::{
    RafxDescriptorIndex, RafxPipelineType, RafxResourceType, RafxResult, RafxRootSignatureDef,
    RafxSampler, MAX_DESCRIPTOR_SET_LAYOUTS,
};
use ash::version::DeviceV1_0;
use ash::vk;
use fnv::FnvHashMap;
use std::sync::Arc;

// Not currently exposed
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct DynamicDescriptorIndex(pub(crate) u32);
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct PushConstantIndex(pub(crate) u32);

#[derive(Clone, Debug)]
pub(crate) struct PushConstantInfo {
    pub(crate) name: Option<String>,
    pub(crate) push_constant_index: PushConstantIndex,
    pub(crate) vk_push_constant_range: vk::PushConstantRange,
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

    // --- vulkan-specific ---
    // Index into DescriptorSetLayoutInfo::dynamic_descriptor_indexes
    pub(crate) dynamic_descriptor_index: Option<DynamicDescriptorIndex>,
    // The index to the first descriptor in the flattened list of all descriptors in the layout
    // none for immutable samplers, which have no update data
    pub(crate) update_data_offset_in_set: Option<u32>,
    pub(crate) has_immutable_sampler: bool,

    pub(crate) vk_type: vk::DescriptorType,
    pub(crate) vk_stages: vk::ShaderStageFlags,
}

#[derive(Default, Debug)]
pub(crate) struct DescriptorSetLayoutInfo {
    // Settable descriptors, immutable samplers are omitted
    pub(crate) descriptors: Vec<RafxDescriptorIndex>,
    // Indexes binding index to the descriptors list
    pub(crate) binding_to_descriptor_index: FnvHashMap<u32, RafxDescriptorIndex>,

    // --- vulkan-specific ---
    pub(crate) update_data_count_per_set: u32,
    // This indexes into the descriptors list
    pub(crate) dynamic_descriptor_indexes: Vec<RafxDescriptorIndex>,
}

#[derive(Debug)]
pub(crate) struct RafxRootSignatureVulkanInner {
    pub(crate) device_context: RafxDeviceContextVulkan,
    pub(crate) pipeline_type: RafxPipelineType,
    pub(crate) layouts: [DescriptorSetLayoutInfo; MAX_DESCRIPTOR_SET_LAYOUTS],
    pub(crate) descriptors: Vec<DescriptorInfo>,
    pub(crate) name_to_descriptor_index: FnvHashMap<String, RafxDescriptorIndex>,
    // Keeps them in scope so they don't drop
    immutable_samplers: Vec<RafxSampler>, //empty_descriptor_sets: [vk::DescriptorSet; MAX_DESCRIPTOR_SETS],

    // --- vulkan-specific ---
    pub(crate) name_to_push_constant_index: FnvHashMap<String, PushConstantIndex>,
    pub(crate) push_constants: Vec<PushConstantInfo>,
    pub(crate) pipeline_layout: vk::PipelineLayout,
    pub(crate) descriptor_set_layouts: [vk::DescriptorSetLayout; MAX_DESCRIPTOR_SET_LAYOUTS],
}

impl Drop for RafxRootSignatureVulkanInner {
    fn drop(&mut self) {
        let device = self.device_context.device();

        unsafe {
            device.destroy_pipeline_layout(self.pipeline_layout, None);

            for &descriptor_set_layout in &self.descriptor_set_layouts {
                device.destroy_descriptor_set_layout(descriptor_set_layout, None);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct RafxRootSignatureVulkan {
    pub(crate) inner: Arc<RafxRootSignatureVulkanInner>,
}

impl RafxRootSignatureVulkan {
    pub fn device_context(&self) -> &RafxDeviceContextVulkan {
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

    pub fn vk_pipeline_layout(&self) -> vk::PipelineLayout {
        self.inner.pipeline_layout
    }

    pub fn vk_descriptor_set_layout(
        &self,
        set_index: u32,
    ) -> Option<vk::DescriptorSetLayout> {
        let layout = self.inner.descriptor_set_layouts[set_index as usize];
        if layout == vk::DescriptorSetLayout::null() {
            None
        } else {
            Some(layout)
        }
    }

    pub fn new(
        device_context: &RafxDeviceContextVulkan,
        root_signature_def: &RafxRootSignatureDef,
    ) -> RafxResult<Self> {
        log::trace!("Create RafxRootSignatureVulkan");

        // If we update this constant, update the arrays in this function
        assert_eq!(MAX_DESCRIPTOR_SET_LAYOUTS, 4);

        let mut push_constants = vec![];
        let mut descriptors = vec![];
        let mut vk_push_constant_ranges = vec![];

        let vk_immutable_samplers: Vec<Vec<vk::Sampler>> = root_signature_def
            .immutable_samplers
            .iter()
            .map(|x| {
                x.samplers
                    .iter()
                    .map(|x| x.vk_sampler().unwrap().vk_sampler())
                    .collect()
            })
            .collect();

        let mut immutable_samplers = vec![];
        for sampler_list in root_signature_def.immutable_samplers {
            for sampler in sampler_list.samplers {
                immutable_samplers.push(sampler.clone());
            }
        }

        // Make sure all shaders are compatible/build lookup of shared data from them
        let (pipeline_type, merged_resources, _merged_resources_name_index_map) =
            crate::internal_shared::merge_resources(root_signature_def)?;

        let mut layouts = [
            DescriptorSetLayoutInfo::default(),
            DescriptorSetLayoutInfo::default(),
            DescriptorSetLayoutInfo::default(),
            DescriptorSetLayoutInfo::default(),
        ];

        let mut vk_set_bindings = [vec![], vec![], vec![], vec![]];

        let mut name_to_descriptor_index = FnvHashMap::default();
        let mut name_to_push_constant_index = FnvHashMap::default();

        //
        // Create bindings (vulkan representation) and descriptors (what we use)
        // We don't create descriptors for immutable samplers
        //
        for resource in &merged_resources {
            let vk_stage_flags = resource.used_in_shader_stages.into();
            let vk_descriptor_type =
                super::util::resource_type_to_descriptor_type(resource.resource_type).unwrap();

            resource.validate()?;

            if resource.resource_type != RafxResourceType::ROOT_CONSTANT {
                // It's not a push constant, so create a vk binding for it
                let mut binding = vk::DescriptorSetLayoutBinding::builder()
                    .binding(resource.binding)
                    .descriptor_count(resource.element_count_normalized())
                    .descriptor_type(vk_descriptor_type)
                    .stage_flags(vk_stage_flags);

                // Determine if flagged as root constant buffer/dynamic uniform buffer. If so, update
                // the type. This was being done by detecting a pattern in the name string. For now
                // this is dead code. It should probably be done by checking the descriptor type.
                // let is_dynamic_uniform_buffer = false;
                // if is_dynamic_uniform_buffer {
                //     if resource.descriptor_count == 1 {
                //         binding =
                //             binding.descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC);
                //     } else {
                //         Err("Cannot use dynamic uniform buffer an array")?;
                //     }
                // }

                let immutable_sampler = crate::internal_shared::find_immutable_sampler_index(
                    root_signature_def.immutable_samplers,
                    &resource.name,
                    resource.set_index,
                    resource.binding,
                );
                if let Some(immutable_sampler_index) = immutable_sampler {
                    if resource.element_count_normalized() as usize
                        != vk_immutable_samplers[immutable_sampler_index].len()
                    {
                        Err(format!(
                            "Descriptor (set={:?} binding={:?}) named {:?} specifies {} elements but the count of provided immutable samplers ({}) did not match",
                            resource.set_index,
                            resource.binding,
                            resource.name,
                            resource.element_count_normalized(),
                            vk_immutable_samplers[immutable_sampler_index].len()
                        ))?;
                    }

                    // immutable_samplers is heap allocated, not modified, and kept in scope. So the
                    // pointer to a value within should remain valid for long enough.
                    binding =
                        binding.immutable_samplers(&vk_immutable_samplers[immutable_sampler_index]);
                }

                let layout: &mut DescriptorSetLayoutInfo =
                    &mut layouts[resource.set_index as usize];

                let vk_bindings: &mut Vec<vk::DescriptorSetLayoutBinding> =
                    &mut vk_set_bindings[resource.set_index as usize];

                if immutable_sampler.is_some()
                    && !resource
                        .resource_type
                        .intersects(RafxResourceType::COMBINED_IMAGE_SAMPLER)
                {
                    // don't expose a immutable sampler unless the image needs to be settable
                    // although we might just not support combined image samplers
                } else if immutable_sampler.is_none()
                    && vk_descriptor_type == vk::DescriptorType::COMBINED_IMAGE_SAMPLER
                {
                    Err(format!(
                        "Descriptor (set={:?} binding={:?}) named {:?} is a combined image sampler but the sampler is NOT immutable. This is not supported. Use separate sampler/image bindings",
                        resource.set_index,
                        resource.binding,
                        resource.name
                    ))?;
                } else {
                    // dynamic storage buffers not supported
                    assert_ne!(
                        binding.descriptor_type,
                        vk::DescriptorType::STORAGE_BUFFER_DYNAMIC
                    );

                    // More than one dynamic descriptor not supported right now
                    assert!(layout.dynamic_descriptor_indexes.is_empty());

                    //
                    // Keep a lookup for dynamic descriptors
                    //
                    let descriptor_index = RafxDescriptorIndex(descriptors.len() as u32);
                    let dynamic_descriptor_index =
                        if binding.descriptor_type == vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC {
                            layout.dynamic_descriptor_indexes.push(descriptor_index);
                            Some(DynamicDescriptorIndex(
                                (layout.dynamic_descriptor_indexes.len() - 1) as u32,
                            ))
                        } else {
                            None
                        };

                    let update_data_offset_in_set = Some(layout.update_data_count_per_set);

                    // Add it to the descriptor list
                    descriptors.push(DescriptorInfo {
                        name: resource.name.clone(),
                        resource_type: resource.resource_type,
                        //texture_dimensions: resource.texture_dimensions,
                        set_index: resource.set_index,
                        binding: resource.binding,
                        element_count: resource.element_count_normalized(),
                        descriptor_index,
                        dynamic_descriptor_index,
                        update_data_offset_in_set,
                        has_immutable_sampler: immutable_sampler.is_some(),
                        vk_type: binding.descriptor_type,
                        vk_stages: binding.stage_flags,
                    });

                    if let Some(name) = resource.name.as_ref() {
                        name_to_descriptor_index.insert(name.clone(), descriptor_index);
                    }

                    layout.descriptors.push(descriptor_index);
                    layout
                        .binding_to_descriptor_index
                        .insert(resource.binding, descriptor_index);

                    layout.update_data_count_per_set += resource.element_count_normalized();
                }

                // Add the binding to the list
                vk_bindings.push(binding.build());
            } else {
                let push_constant_index = PushConstantIndex(push_constants.len() as u32);
                let vk_push_constant_range = vk::PushConstantRange::builder()
                    .offset(0)
                    .size(resource.size_in_bytes)
                    .stage_flags(vk_stage_flags)
                    .build();

                // it's a push constant
                let push_constant = PushConstantInfo {
                    name: resource.name.clone(),
                    push_constant_index,
                    vk_push_constant_range,
                };

                push_constants.push(push_constant);
                vk_push_constant_ranges.push(vk_push_constant_range);
                if let Some(name) = resource.name.as_ref() {
                    name_to_push_constant_index.insert(name.clone(), push_constant_index);
                }
            }
        }

        //
        // Create descriptor set layouts
        //
        let mut descriptor_set_layouts =
            [vk::DescriptorSetLayout::null(); MAX_DESCRIPTOR_SET_LAYOUTS];
        let mut descriptor_set_layout_count = 0;

        for layout_index in 0..MAX_DESCRIPTOR_SET_LAYOUTS {
            let vk_bindings: &mut Vec<vk::DescriptorSetLayoutBinding> =
                &mut vk_set_bindings[layout_index as usize];

            //
            // Layout is empty, skip it
            //
            if vk_bindings.is_empty() {
                continue;
            }

            //
            // Fill in any sets we skipped with empty sets to ensure this layout is indexable by set
            // index and to make vulkan happy
            //
            while descriptor_set_layout_count < layout_index {
                let descriptor_set_layout = unsafe {
                    device_context.device().create_descriptor_set_layout(
                        &*vk::DescriptorSetLayoutCreateInfo::builder(),
                        None,
                    )?
                };

                descriptor_set_layouts[descriptor_set_layout_count] = descriptor_set_layout;
                descriptor_set_layout_count += 1;
            }

            //
            // Create this layout
            //
            {
                let descriptor_set_layout = unsafe {
                    device_context.device().create_descriptor_set_layout(
                        &*vk::DescriptorSetLayoutCreateInfo::builder().bindings(&vk_bindings),
                        None,
                    )?
                };

                descriptor_set_layouts[descriptor_set_layout_count] = descriptor_set_layout;
                descriptor_set_layout_count += 1;
            };
        }

        //
        // Create pipeline layout
        //
        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&descriptor_set_layouts[0..descriptor_set_layout_count])
            .push_constant_ranges(&vk_push_constant_ranges);

        let pipeline_layout = unsafe {
            device_context
                .device()
                .create_pipeline_layout(&pipeline_layout_create_info, None)?
        };

        //TODO: Support update templates

        let inner = RafxRootSignatureVulkanInner {
            device_context: device_context.clone(),
            pipeline_type,
            layouts,
            descriptors,
            name_to_descriptor_index,
            immutable_samplers,
            push_constants,
            pipeline_layout,
            descriptor_set_layouts,
            name_to_push_constant_index,
        };

        Ok(RafxRootSignatureVulkan {
            inner: Arc::new(inner),
        })
    }
}
