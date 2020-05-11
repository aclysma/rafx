
use ash::vk;
use crate::asset_storage::ResourceHandle;
use image2::Hash;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use ash::prelude::VkResult;
use ash::version::DeviceV1_0;
use fnv::FnvHashMap;
use std::collections::hash_map::Entry::Occupied;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DescriptorType {
    SAMPLER,
    COMBINED_IMAGE_SAMPLER,
    SAMPLED_IMAGE,
    STORAGE_IMAGE,
    UNIFORM_TEXEL_BUFFER,
    STORAGE_TEXEL_BUFFER,
    UNIFORM_BUFFER,
    STORAGE_BUFFER,
    UNIFORM_BUFFER_DYNAMIC,
    STORAGE_BUFFER_DYNAMIC,
    INPUT_ATTACHMENT,
}

impl Into<vk::DescriptorType> for DescriptorType {
    fn into(self) -> vk::DescriptorType {
        match self {
            SAMPLER => vk::DescriptorType::SAMPLER,
            COMBINED_IMAGE_SAMPLER => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            SAMPLED_IMAGE => vk::DescriptorType::SAMPLED_IMAGE,
            STORAGE_IMAGE => vk::DescriptorType::STORAGE_IMAGE,
            UNIFORM_TEXEL_BUFFER => vk::DescriptorType::UNIFORM_TEXEL_BUFFER,
            STORAGE_TEXEL_BUFFER => vk::DescriptorType::STORAGE_TEXEL_BUFFER,
            UNIFORM_BUFFER => vk::DescriptorType::UNIFORM_BUFFER,
            STORAGE_BUFFER => vk::DescriptorType::STORAGE_BUFFER,
            UNIFORM_BUFFER_DYNAMIC => vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC,
            STORAGE_BUFFER_DYNAMIC => vk::DescriptorType::STORAGE_BUFFER_DYNAMIC,
            INPUT_ATTACHMENT => vk::DescriptorType::INPUT_ATTACHMENT,
        }
    }
}

impl Default for DescriptorType {
    fn default() -> Self {
        DescriptorType::SAMPLER
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ShaderStageFlags {
    VERTEX,
    TESSELLATION_CONTROL,
    TESSELLATION_EVALUATION,
    GEOMETRY,
    FRAGMENT,
    COMPUTE,
    ALL_GRAPHICS,
    ALL,
}

impl Into<vk::ShaderStageFlags> for ShaderStageFlags {
    fn into(self) -> vk::ShaderStageFlags {
        match self {
            VERTEX => vk::ShaderStageFlags::VERTEX,
            TESSELLATION_CONTROL => vk::ShaderStageFlags::TESSELLATION_CONTROL,
            TESSELLATION_EVALUATION => vk::ShaderStageFlags::TESSELLATION_EVALUATION,
            GEOMETRY => vk::ShaderStageFlags::GEOMETRY,
            FRAGMENT => vk::ShaderStageFlags::FRAGMENT,
            COMPUTE => vk::ShaderStageFlags::COMPUTE,
            ALL_GRAPHICS => vk::ShaderStageFlags::ALL_GRAPHICS,
            ALL => vk::ShaderStageFlags::ALL,
        }
    }
}

impl Default for ShaderStageFlags {
    fn default() -> Self {
        ShaderStageFlags::VERTEX
    }
}


// #[derive(Debug, Copy, Clone, PartialEq, Hash)]
// enum SamplerCreateInfo {
//     //TODO: Fill in fields as they are needed
// }
//
// impl DescriptorType {
//     fn as_builder(&self) -> vk::SamplerCreateInfoBuilder {
//         vk::SamplerCreateInfo::builder()
//     }
// }

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DescriptorSetLayoutBinding {
    pub binding: u32,
    pub descriptor_type: DescriptorType,
    pub descriptor_count: u32,
    pub stage_flags: ShaderStageFlags,
    //samplers: Vec<SamplerCreateInfo>,
}

impl DescriptorSetLayoutBinding {
    pub fn as_builder(&self) -> vk::DescriptorSetLayoutBindingBuilder {
        vk::DescriptorSetLayoutBinding::builder()
            .binding(self.binding)
            .descriptor_type(self.descriptor_type.into())
            .descriptor_count(self.descriptor_count)
            .stage_flags(self.stage_flags.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DescriptorSetLayout {
    pub descriptor_set_layout_bindings: Vec<DescriptorSetLayoutBinding>,
}

impl DescriptorSetLayout {
    pub fn new() -> Self {
        DescriptorSetLayout {
            descriptor_set_layout_bindings: Default::default()
        }
    }

    // pub unsafe fn to_vk(&self, device: &ash::Device) -> VkResult<vk::DescriptorSetLayout> {
    //     let bindings : Vec<_> = self.descriptor_set_layout_bindings.iter()
    //         .map(|binding| binding.as_builder().build())
    //         .collect();
    //
    //     let create_info = vk::DescriptorSetLayoutCreateInfo::builder()
    //         .bindings(&bindings);
    //
    //     device.create_descriptor_set_layout(&*create_info, None)
    // }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PushConstantRange {
    pub stage_flags: ShaderStageFlags,
    pub offset: u32,
    pub size: u32,
}


impl PushConstantRange {
    pub fn as_builder(&self) -> vk::PushConstantRangeBuilder {
        vk::PushConstantRange::builder()
            .stage_flags(self.stage_flags.into())
            .offset(self.offset)
            .size(self.size)
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PipelineLayout {
    pub descriptor_set_layouts: Vec<DescriptorSetLayout>,
    pub push_constant_ranges: Vec<PushConstantRange>,
}

impl PipelineLayout {
    pub fn new() -> Self {
        PipelineLayout {
            descriptor_set_layouts: Default::default(),
            push_constant_ranges: Default::default(),
        }
    }
/*
    pub unsafe fn to_vk(
        &self,
        descriptor_set_layout_cache: &mut FnvHashMap<DescriptorSetLayout, vk::DescriptorSetLayout>,
        device: &ash::Device
    ) -> VkResult<vk::PipelineLayout> {
        let mut descriptor_set_layouts = Vec::with_capacity(self.descriptor_set_layouts.len());
        for descriptor_set_layout in &self.descriptor_set_layouts {
            let entry = descriptor_set_layout_cache
                .entry(descriptor_set_layout.clone());

            if let Occupied(entry) = entry {
                descriptor_set_layouts.push(*entry.get());
            } else {
                let vk_obj = descriptor_set_layout.to_vk(device)?;
                descriptor_set_layout_cache.insert(descriptor_set_layout.clone(), vk_obj);
                descriptor_set_layouts.push(vk_obj);
            }
        }

        let push_constant_ranges : Vec<_> = self.push_constant_ranges.iter()
            .map(|binding| binding.as_builder().build())
            .collect();

        let create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(descriptor_set_layouts.as_slice())
            .push_constant_ranges(push_constant_ranges.as_slice());

        device.create_pipeline_layout(&*create_info, None)
    }
    */
}




// struct PipelineLayoutDefinition {
//
//
//     descriptor_set_layout_definitions:
//     //descriptor_set_layout_definitions: Vec<ResourceHandle<DescriptorSetLayoutDefinition>>,
//     push_constant_ranges: Vec<vk::PushConstantRange>,
//     flags: vk::PipelineLayoutCreateFlags
// }
//
// impl PipelineLayoutDefinition {
//     pub fn new() -> Self {
//         PipelineLayoutDefinition {
//             descriptor_set_layout_bindings: Default::default(),
//             push_constant_ranges: Default::default(),
//             flags: Default::default()
//         }
//     }
//
//     pub fn create_info(&self, descriptor_set_layouts: &[vk::DescriptorSetLayout]) -> vk::PipelineLayoutCreateInfo {
//         vk::PipelineLayoutCreateInfo::builder()
//             .set_layouts(self.descriptor_set_layout_bindings.as_slice())
//             .push_constant_ranges(self.push_constant_ranges)
//             .flags(self.flags)
//     }
// }
//
//
//
// struct PipelineLayout {
//
// }
//
// impl PipelineLayout {
//
// }
//
//
//


struct AttachmentDescription {

}


/*

        let renderpass_attachments = [vk::AttachmentDescription::builder()
            .format(swapchain_info.surface_format.format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .build()];

        let color_attachment_refs = [vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];

        let subpasses = [vk::SubpassDescription::builder()
            .color_attachments(&color_attachment_refs)
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .build()];

        let dependencies = [vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::default())
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(
                vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            )
            .build()];

        let renderpass_create_info = vk::RenderPassCreateInfo::builder()
            .attachments(&renderpass_attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);
*/