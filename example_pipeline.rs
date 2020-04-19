
use std::mem;
use ash::vk;
use std::ffi::CString;
use std::mem::ManuallyDrop;

use ash::version::DeviceV1_0;

use super::VkDevice;
use super::VkSwapchain;
use crate::offset_of;
use super::SwapchainInfo;
use super::QueueFamilyIndices;
use crate::renderer::{VkBuffer, VkImage};

#[derive(Clone, Debug, Copy)]
struct UniformBufferObject {
    model: glam::Mat4,
    view: glam::Mat4,
    proj: glam::Mat4
}

#[derive(Clone, Debug, Copy)]
struct Vertex {
    pos: [f32; 2],
    color: [f32; 3],
    tex_coord: [f32; 2]
}

const VERTEX_LIST : [Vertex; 4] = [
    Vertex {
        pos: [-0.5, -0.5],
        color: [1.0, 0.0, 0.0],
        tex_coord: [1.0, 0.0]
    },
    Vertex {
        pos: [0.5, -0.5],
        color: [1.0, 1.0, 0.0],
        tex_coord: [0.0, 0.0]

    },
    Vertex {
        pos: [0.5, 0.5],
        color: [1.0, 0.0, 1.0],
        tex_coord: [0.0, 1.0]
    },
    Vertex {
        pos: [-0.5, 0.5],
        color: [0.0, 0.0, 1.0],
        tex_coord: [1.0, 1.0]
    }
];

const INDEX_LIST : [u16; 6] = [0, 1, 2, 2, 3, 0];

struct FixedFunctionState<'a> {
    vertex_input_assembly_state_info: vk::PipelineInputAssemblyStateCreateInfoBuilder<'a>,
    vertex_input_state_info: vk::PipelineVertexInputStateCreateInfoBuilder<'a>,
    viewport_state_info: vk::PipelineViewportStateCreateInfoBuilder<'a>,
    rasterization_info: vk::PipelineRasterizationStateCreateInfoBuilder<'a>,
    multisample_state_info: vk::PipelineMultisampleStateCreateInfoBuilder<'a>,
    color_blend_state_info: vk::PipelineColorBlendStateCreateInfoBuilder<'a>,
    dynamic_state_info: vk::PipelineDynamicStateCreateInfoBuilder<'a>
}

struct PipelineResources {
    pipeline_layout : vk::PipelineLayout,
    renderpass : vk::RenderPass,
    pipeline : vk::Pipeline
}

pub struct VkPipeline {
    pub device : ash::Device, // This struct is not responsible for releasing this
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub pipeline_layout : vk::PipelineLayout,
    pub renderpass : vk::RenderPass,
    pub pipeline : vk::Pipeline,
    pub frame_buffers : Vec<vk::Framebuffer>,
    pub command_pool : vk::CommandPool,
    pub command_buffers : Vec<vk::CommandBuffer>,
    pub vertex_buffer: ManuallyDrop<VkBuffer>,
    pub index_buffer: ManuallyDrop<VkBuffer>,
    pub uniform_buffers: Vec<ManuallyDrop<VkBuffer>>,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub image: ManuallyDrop<VkImage>,
    pub image_view: vk::ImageView,
    pub image_sampler: vk::Sampler
}

impl VkPipeline {
    pub fn new(device: &VkDevice, swapchain: &VkSwapchain) -> Self {
        let mut pipeline_resources = None;

        let descriptor_set_layout = Self::create_descriptor_set_layout(&device.logical_device);

        Self::create_fixed_function_state(&swapchain.swapchain_info, |fixed_function_state| {
            Self::create_renderpass_create_info(&swapchain.swapchain_info, |renderpass_create_info| {
                Self::create_pipeline(
                    &device.logical_device,
                    &swapchain.swapchain_info,
                    fixed_function_state,
                    renderpass_create_info,
                    descriptor_set_layout,
                    |resources| {
                        pipeline_resources = Some(resources);
                    }
                );
            });
        });

        let pipeline_resources = pipeline_resources.unwrap();
        let pipeline_layout = pipeline_resources.pipeline_layout;
        let renderpass = pipeline_resources.renderpass;
        let pipeline = pipeline_resources.pipeline;

        let frame_buffers = Self::create_framebuffers(
            &device.logical_device,
            &swapchain.swapchain_image_views,
            &swapchain.swapchain_info,
            &pipeline_resources.renderpass
        );

        let command_pool = Self::create_command_pool(
            &device.logical_device,
            &device.queue_family_indices
        );

        let command_buffers = Self::create_command_buffers(
            &device.logical_device,
            &swapchain.swapchain_info,
            &command_pool
        );

        let vertex_buffer = Self::create_vertex_buffer(
            &device.logical_device,
            &device.queues.graphics_queue,
            &command_pool,
            &device.memory_properties
        );

        let index_buffer = Self::create_index_buffer(
            &device.logical_device,
            &device.queues.graphics_queue,
            &command_pool,
            &device.memory_properties
        );

        let uniform_buffers : Vec<_> = (0..swapchain.swapchain_info.image_count).map(|_| {
            Self::create_uniform_buffer(&device.logical_device, &device.memory_properties)
        }).collect();

        let image = Self::load_image(
            &device.logical_device,
            &device.queues.graphics_queue,
            &command_pool,
            &device.memory_properties
        );

        let image_view = Self::create_texture_image_view(
            &device.logical_device,
            &image.image
        );

        let image_sampler = Self::create_texture_image_sampler(
            &device.logical_device
        );

        let descriptor_pool = Self::create_descriptor_pool(
            &device.logical_device,
            swapchain.swapchain_info.image_count as u32
        );

        let descriptor_sets = Self::create_descriptor_sets(
            &device.logical_device,
            &descriptor_pool,
            &descriptor_set_layout,
            swapchain.swapchain_info.image_count,
            &uniform_buffers,
            &image_view,
            &image_sampler
        );

        for i in 0..swapchain.swapchain_info.image_count {
            Self::record_command_buffer(
                &device.logical_device,
                &swapchain.swapchain_info,
                &renderpass,
                &frame_buffers[i],
                &pipeline,
                &pipeline_layout,
                &command_buffers[i],
                &vertex_buffer.buffer,
                &index_buffer.buffer,
                &descriptor_sets[i]
            );
        }

        VkPipeline {
            device: device.logical_device.clone(),
            descriptor_set_layout,
            pipeline_layout,
            renderpass,
            pipeline,
            frame_buffers,
            command_pool,
            command_buffers,
            vertex_buffer,
            index_buffer,
            uniform_buffers,
            descriptor_pool,
            descriptor_sets,
            image,
            image_view,
            image_sampler
        }
    }

    fn create_descriptor_set_layout(
        logical_device: &ash::Device
    )
        -> vk::DescriptorSetLayout
    {
        let descriptor_set_layout_bindings = [
            vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .build(),

            vk::DescriptorSetLayoutBinding::builder()
                .binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .build(),
        ];

        let descriptor_set_layout_create_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&descriptor_set_layout_bindings);

        unsafe {
            logical_device.create_descriptor_set_layout(&descriptor_set_layout_create_info, None).unwrap()
        }
    }

    fn create_fixed_function_state<F : FnMut(&FixedFunctionState)>(swapchain_info: &SwapchainInfo, mut f: F) {
        let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let vertex_input_binding_descriptions = [
            vk::VertexInputBindingDescription {
                binding: 0,
                stride: mem::size_of::<Vertex>() as u32,
                input_rate: vk::VertexInputRate::VERTEX,
            }
        ];
        let vertex_input_attribute_descriptions = [
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex, color) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex, tex_coord) as u32,
            },
        ];

        let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(&vertex_input_attribute_descriptions)
            .vertex_binding_descriptions(&vertex_input_binding_descriptions);

        let viewports = [
            vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: swapchain_info.extents.width as f32,
                height: swapchain_info.extents.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            }
        ];

        let scissors = [vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: swapchain_info.extents.clone(),
        }];

        let viewport_state_info = vk::PipelineViewportStateCreateInfo::builder()
            .scissors(&scissors)
            .viewports(&viewports);

        let rasterization_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .line_width(1.0)
            .polygon_mode(vk::PolygonMode::FILL);

        // Skip depth/stencil testing

        let multisample_state_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        // Applies to the current framebuffer
        let color_blend_attachment_states = [
            vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(vk::ColorComponentFlags::all())
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .alpha_blend_op(vk::BlendOp::ADD)
                .build()
        ];

        // Applies globally
        let color_blend_state_info = vk::PipelineColorBlendStateCreateInfo::builder()
            .attachments(&color_blend_attachment_states);

        let dynamic_state = vec![/*vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR, vk::DynamicState::LINE_WIDTH*/];
        let dynamic_state_info = vk::PipelineDynamicStateCreateInfo::builder()
            .dynamic_states(&dynamic_state);

        let fixed_function_state = FixedFunctionState {
            vertex_input_assembly_state_info,
            vertex_input_state_info,
            viewport_state_info,
            rasterization_info,
            multisample_state_info,
            color_blend_state_info,
            dynamic_state_info
        };

        f(&fixed_function_state);
    }

    fn create_renderpass_create_info<F : FnMut(&vk::RenderPassCreateInfo)>(swapchain_info: &SwapchainInfo, mut f: F) {
        let renderpass_attachments = [
            vk::AttachmentDescription::builder()
                .format(swapchain_info.surface_format.format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .build()
        ];

        let color_attachment_refs = [vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];

        let subpasses = [
            vk::SubpassDescription::builder()
                .color_attachments(&color_attachment_refs)
                .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                .build()
        ];

        let dependencies = [
            vk::SubpassDependency::builder()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .dst_subpass(0)
                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .src_access_mask(vk::AccessFlags::default())
                .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .build()
        ];

        let renderpass_create_info = vk::RenderPassCreateInfo::builder()
            .attachments(&renderpass_attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);

        f(&renderpass_create_info);
    }

    fn create_pipeline<F : FnMut(PipelineResources)>(
        logical_device: &ash::Device,
        _swapchain_info: &SwapchainInfo,
        fixed_function_state: &FixedFunctionState,
        renderpass_create_info: &vk::RenderPassCreateInfo,
        descriptor_set_layout: vk::DescriptorSetLayout,
        mut f: F
    ) {
        //
        // Load Shaders
        //
        let vertex_shader_module = Self::load_shader_module(
            logical_device,
            &include_bytes!("../../assets/shaders/tutorial/tutorial.vert.spv")[..]);

        let fragment_shader_module = Self::load_shader_module(
            logical_device,
            &include_bytes!("../../assets/shaders/tutorial/tutorial.frag.spv")[..]);

        let shader_entry_name = CString::new("main").unwrap();
        let shader_stage_create_infos = [
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vertex_shader_module)
                .name(&shader_entry_name)
                .build(),

            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(fragment_shader_module)
                .name(&shader_entry_name)
                .build()
        ];

        let descriptor_set_layouts = [
            descriptor_set_layout
        ];

        let layout_create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&descriptor_set_layouts);

        let pipeline_layout : vk::PipelineLayout = unsafe {
            logical_device
                .create_pipeline_layout(&layout_create_info, None)
                .unwrap()
        };

        let renderpass : vk::RenderPass = unsafe {
            logical_device
                .create_render_pass(renderpass_create_info, None)
                .unwrap()
        };

        let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stage_create_infos)
            .vertex_input_state(&fixed_function_state.vertex_input_state_info)
            .input_assembly_state(&fixed_function_state.vertex_input_assembly_state_info)
            .viewport_state(&fixed_function_state.viewport_state_info)
            .rasterization_state(&fixed_function_state.rasterization_info)
            .multisample_state(&fixed_function_state.multisample_state_info)
            .color_blend_state(&fixed_function_state.color_blend_state_info)
            .dynamic_state(&fixed_function_state.dynamic_state_info)
            .layout(pipeline_layout)
            .render_pass(renderpass);

        let pipelines : Vec<vk::Pipeline> = unsafe {
            logical_device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[pipeline_info.build()],
                    None,
                )
                .expect("Unable to create graphics pipeline")
        };
        let pipeline = pipelines[0];

        //
        // Destroy shader modules. They don't need to be kept around once the pipeline is built
        //
        unsafe {
            logical_device.destroy_shader_module(vertex_shader_module, None);
            logical_device.destroy_shader_module(fragment_shader_module, None);
        }

        let resources = PipelineResources {
            pipeline_layout,
            renderpass,
            pipeline
        };

        f(resources);
    }

    fn load_shader_module(logical_device: &ash::Device, data: &[u8]) -> vk::ShaderModule {
        let mut spv_file = std::io::Cursor::new(data);
        let code = super::util::read_spv(&mut spv_file).expect("Failed to read vertex shader spv file");
        let shader_info = vk::ShaderModuleCreateInfo::builder().code(&code);

        unsafe {
            logical_device
                .create_shader_module(&shader_info, None)
                .expect("Vertex shader module error")
        }
    }

    fn create_framebuffers(
        logical_device: &ash::Device,
        swapchain_image_views: &Vec<vk::ImageView>,
        swapchain_info: &SwapchainInfo,
        renderpass: &vk::RenderPass
    )
        -> Vec<vk::Framebuffer>
    {
        swapchain_image_views
            .iter()
            .map(|&swapchain_image_view| {
                let framebuffer_attachments = [swapchain_image_view];
                let frame_buffer_create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(*renderpass)
                    .attachments(&framebuffer_attachments)
                    .width(swapchain_info.extents.width)
                    .height(swapchain_info.extents.height)
                    .layers(1);

                unsafe {
                    logical_device
                        .create_framebuffer(&frame_buffer_create_info, None)
                        .unwrap()
                }
            })
            .collect()
    }

    fn create_command_pool(
        logical_device: &ash::Device,
        queue_family_indices: &QueueFamilyIndices
    )
        -> vk::CommandPool
    {
        let pool_create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_family_indices.graphics_queue_family_index);

        unsafe {
            logical_device.create_command_pool(&pool_create_info, None).unwrap()
        }
    }

    fn create_command_buffers(
        logical_device: &ash::Device,
        swapchain_info: &SwapchainInfo,
        command_pool: &vk::CommandPool
    )
        -> Vec<vk::CommandBuffer>
    {
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(swapchain_info.image_count as u32)
            .command_pool(*command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        unsafe {
            logical_device
                .allocate_command_buffers(&command_buffer_allocate_info)
                .unwrap()
        }
    }

    fn create_vertex_buffer(
        logical_device: &ash::Device,
        queue: &vk::Queue,
        command_pool: &vk::CommandPool,
        device_memory_properties: &vk::PhysicalDeviceMemoryProperties
    )
        -> ManuallyDrop<VkBuffer>
    {
        VkBuffer::new_from_slice_device_local(
            logical_device,
            device_memory_properties,
            queue,
            command_pool,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            &VERTEX_LIST)
    }

    fn create_index_buffer(
        logical_device: &ash::Device,
        queue: &vk::Queue,
        command_pool: &vk::CommandPool,
        device_memory_properties: &vk::PhysicalDeviceMemoryProperties
    )
        -> ManuallyDrop<VkBuffer>
    {
        VkBuffer::new_from_slice_device_local(
            logical_device,
            device_memory_properties,
            queue,
            command_pool,
            vk::BufferUsageFlags::INDEX_BUFFER,
            &INDEX_LIST)
    }

    fn create_uniform_buffer(
        logical_device: &ash::Device,
        device_memory_properties: &vk::PhysicalDeviceMemoryProperties
    )
        -> ManuallyDrop<VkBuffer>
    {
        let buffer = VkBuffer::new(
            logical_device,
            device_memory_properties,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            mem::size_of::<UniformBufferObject>() as u64);

        ManuallyDrop::new(buffer)
    }

    fn create_descriptor_pool(
        logical_device: &ash::Device,
        swapchain_image_count: u32
    )
        -> vk::DescriptorPool
    {
        let pool_sizes = [
            vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(swapchain_image_count)
                .build(),
            vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(swapchain_image_count)
                .build()
        ];

        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_sizes)
            .max_sets(swapchain_image_count);

        unsafe {
            logical_device.create_descriptor_pool(&descriptor_pool_info, None).unwrap()
        }
    }

    fn create_descriptor_sets(
        logical_device: &ash::Device,
        descriptor_pool: &vk::DescriptorPool,
        descriptor_set_layout: &vk::DescriptorSetLayout,
        swapchain_image_count: usize,
        uniform_buffers: &Vec<ManuallyDrop<VkBuffer>>,
        image_view: &vk::ImageView,
        sampler: &vk::Sampler
    )
        -> Vec<vk::DescriptorSet>
    {
        // DescriptorSetAllocateInfo expects an array with an element per set
        let descriptor_set_layouts = vec![*descriptor_set_layout; swapchain_image_count];

        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(*descriptor_pool)
            .set_layouts(descriptor_set_layouts.as_slice());

        let descriptor_sets = unsafe {
            logical_device.allocate_descriptor_sets(&alloc_info).unwrap()
        };

        for i in 0..swapchain_image_count {
            let descriptor_buffer_infos = [
                vk::DescriptorBufferInfo::builder()
                    .buffer(uniform_buffers[i].buffer)
                    .offset(0)
                    .range(mem::size_of::<UniformBufferObject>() as u64)
                    .build()
            ];

            let descriptor_image_infos = [
                vk::DescriptorImageInfo::builder()
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image_view(*image_view)
                    .sampler(*sampler)
                    .build(),
            ];

            let descriptor_writes = [
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_sets[i])
                    .dst_binding(0)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(&descriptor_buffer_infos)
                    .build(),

                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_sets[i])
                    .dst_binding(1)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&descriptor_image_infos)
                    .build(),
            ];

            unsafe {
                logical_device.update_descriptor_sets(&descriptor_writes, &[]);
            }
        }

        descriptor_sets
    }

    fn record_command_buffer(
        logical_device: &ash::Device,
        swapchain_info: &SwapchainInfo,
        renderpass: &vk::RenderPass,
        framebuffer: &vk::Framebuffer,
        pipeline: &vk::Pipeline,
        pipeline_layout: &vk::PipelineLayout,
        command_buffer: &vk::CommandBuffer,
        vertex_buffer: &vk::Buffer,
        index_buffer: &vk::Buffer,
        descriptor_set: &vk::DescriptorSet
    ) {
        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder();

        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            },
        ];

        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(*renderpass)
            .framebuffer(*framebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: swapchain_info.extents.clone(),
            })
            .clear_values(&clear_values);

        // Implicitly resets the command buffer
        unsafe {
            logical_device
                .begin_command_buffer(*command_buffer, &command_buffer_begin_info)
                .expect("Begin commandbuffer");

            logical_device.cmd_begin_render_pass(
                *command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );

            logical_device.cmd_bind_pipeline(
                *command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                *pipeline,
            );

            logical_device.cmd_bind_vertex_buffers(
                *command_buffer,
                0, // first binding
                &[*vertex_buffer],
                &[0], // offsets
            );

            logical_device.cmd_bind_index_buffer(
                *command_buffer,
                *index_buffer,
                0, // offset
                vk::IndexType::UINT16
            );

            logical_device.cmd_bind_descriptor_sets(
                *command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                *pipeline_layout,
                0,
                &[*descriptor_set],
                &[]
            );

            //logical_device.cmd_draw(*command_buffer, 3, 1, 0, 0);
            logical_device.cmd_draw_indexed(*command_buffer, INDEX_LIST.len() as u32, 1, 0, 0, 0);
            logical_device.cmd_end_render_pass(*command_buffer);

            logical_device.end_command_buffer(*command_buffer).unwrap();
        }
    }

    pub fn update_uniform_buffer(
        &mut self,
        time_state: &super::TimeState,
        swapchain_image_index: u32,
        _extents: vk::Extent2D
    ) {
        let time_f32 = time_state.system().total_time.as_secs_f32();
        let ubo = UniformBufferObject {
            model: glam::Mat4::from_rotation_z(glam::Angle::from_degrees(time_f32 * 20.0)),
            view: glam::Mat4::identity(),
            proj: glam::Mat4::identity()
        };

        self.uniform_buffers[swapchain_image_index as usize].write_to_host_visible_buffer(&[ubo]);
    }

    pub fn load_image(
        logical_device: &ash::Device,
        queue: &vk::Queue,
        command_pool: &vk::CommandPool,
        device_memory_properties: &vk::PhysicalDeviceMemoryProperties
    )
        -> ManuallyDrop<VkImage>
    {
        let loaded_image = image::load_from_memory(include_bytes!("../../assets/textures/texture.jpg"))
            .unwrap()
            .to_rgba();
        let (width, height) = loaded_image.dimensions();
        let extent = vk::Extent3D {
            width,
            height,
            depth: 1
        };
        let image_data = loaded_image.into_raw();

        let mut staging_buffer = VkBuffer::new(
            logical_device,
            device_memory_properties,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            image_data.len() as u64);

        staging_buffer.write_to_host_visible_buffer(&image_data);

        let image = VkImage::new(
            logical_device,
            device_memory_properties,
            extent,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL);

        super::util::transition_image_layout(
            logical_device,
            queue,
            command_pool,
            &image.image,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL);

        super::util::copy_buffer_to_image(
            logical_device,
            queue,
            command_pool,
            &staging_buffer.buffer,
            &image.image,
            &image.extent
        );

        super::util::transition_image_layout(
            logical_device,
            queue,
            command_pool,
            &image.image,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

        ManuallyDrop::new(image)
    }

    pub fn create_texture_image_view(
        logical_device: &ash::Device,
        image: &vk::Image
    )
        -> vk::ImageView
    {
        let subresource_range = vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1);

        let image_view_info = vk::ImageViewCreateInfo::builder()
            .image(*image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .subresource_range(*subresource_range);

        unsafe {
            logical_device.create_image_view(&image_view_info, None).unwrap()
        }
    }

    pub fn create_texture_image_sampler(
        logical_device: &ash::Device
    )
        -> vk::Sampler
    {
        let sampler_info = vk::SamplerCreateInfo::builder()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(true)
            .max_anisotropy(16.0)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .mip_lod_bias(0.0)
            .min_lod(0.0)
            .max_lod(0.0);

        unsafe {
            logical_device.create_sampler(&sampler_info, None).unwrap()
        }
    }
}

impl Drop for VkPipeline {
    fn drop(&mut self) {
        info!("destroying VkPipeline");

        unsafe {
            self.device.destroy_sampler(self.image_sampler, None);
            self.device.destroy_image_view(self.image_view, None);
            ManuallyDrop::drop(&mut self.image);

            for uniform_buffer in &mut self.uniform_buffers {
                ManuallyDrop::drop(uniform_buffer);
            }

            ManuallyDrop::drop(&mut self.vertex_buffer);
            ManuallyDrop::drop(&mut self.index_buffer);

            self.device.destroy_command_pool(self.command_pool, None);

            for frame_buffer in &self.frame_buffers {
                self.device.destroy_framebuffer(*frame_buffer, None);
            }

            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_render_pass(self.renderpass, None);

            self.device.destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }

        info!("destroyed VkPipeline");
    }
}