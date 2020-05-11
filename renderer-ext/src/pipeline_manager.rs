
use crate::pipeline_description as dsc;
use fnv::FnvHashMap;
use ash::vk;
use ash::version::DeviceV1_0;
use renderer_shell_vulkan::VkDeviceContext;
use ash::prelude::VkResult;
use std::collections::hash_map::Entry::Occupied;

struct DescriptorSetLayoutState {
    vk_obj: vk::DescriptorSetLayout
}

struct PipelineLayoutState {
    vk_obj: vk::PipelineLayout
}

struct PipelineManager {
    device_context: VkDeviceContext,
    descriptor_set_layouts: FnvHashMap<dsc::DescriptorSetLayout, DescriptorSetLayoutState>,
    pipeline_layouts: FnvHashMap<dsc::PipelineLayout, PipelineLayoutState>
}

impl PipelineManager {
    fn new(device_context: &VkDeviceContext) -> Self {
        PipelineManager {
            device_context: device_context.clone(),
            descriptor_set_layouts: Default::default(),
            pipeline_layouts: Default::default()
        }
    }

    pub unsafe fn get_or_create_descriptor_set_layout(
        &mut self,
        descriptor_set_layout: &dsc::DescriptorSetLayout
    ) -> VkResult<vk::DescriptorSetLayout> {
        let entry = self.descriptor_set_layouts
            .entry(descriptor_set_layout.clone());

        if let Occupied(entry) = entry {
            return Ok(entry.get().vk_obj);
        } else {
            let bindings : Vec<_> = descriptor_set_layout.descriptor_set_layout_bindings.iter()
                .map(|binding| binding.as_builder().build())
                .collect();

            let create_info = vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&bindings);

            let vk_obj = self.device_context.device().create_descriptor_set_layout(&*create_info, None)?;
            entry.or_insert(DescriptorSetLayoutState {
                vk_obj
            });
            Ok(vk_obj)
        }
    }

    pub unsafe fn get_or_create_pipeline_layout(
        &mut self,
        pipeline_layout: &dsc::PipelineLayout
    ) -> VkResult<vk::PipelineLayout> {
        if let Some(pipeline_layout_state) = self.pipeline_layouts.get(pipeline_layout) {
            return Ok(pipeline_layout_state.vk_obj);
        } else {
            let mut descriptor_set_layouts = Vec::with_capacity(pipeline_layout.descriptor_set_layouts.len());
            for descriptor_set_layout in &pipeline_layout.descriptor_set_layouts {
                descriptor_set_layouts.push(self.get_or_create_descriptor_set_layout(descriptor_set_layout)?);
            }

            let push_constant_ranges : Vec<_> = pipeline_layout.push_constant_ranges.iter()
                .map(|push_constant_range| push_constant_range.as_builder().build())
                .collect();

            let create_info = vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(descriptor_set_layouts.as_slice())
                .push_constant_ranges(push_constant_ranges.as_slice());

            let vk_obj = self.device_context.device().create_pipeline_layout(&*create_info, None)?;
            self.pipeline_layouts.insert(pipeline_layout.clone(), PipelineLayoutState {
                vk_obj
            });
            Ok(vk_obj)
        }
    }
}