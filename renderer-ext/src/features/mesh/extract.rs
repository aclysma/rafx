use crate::features::mesh::{ExtractedFrameNodeMeshData, MeshRenderNodeSet, MeshRenderFeature, MeshRenderNode, MeshDrawCall, MeshPerObjectShaderParam, ExtractedViewNodeMeshData};
use crate::{RenderJobExtractContext, PositionComponent, MeshComponent, RenderJobWriteContext, RenderJobPrepareContext};
use renderer_base::{DefaultExtractJobImpl, FramePacket, RenderView, PerViewNode, PrepareJob, DefaultPrepareJob, RenderFeatureIndex, RenderFeature, PerFrameNode};
use renderer_base::slab::RawSlabKey;
use crate::features::mesh::prepare::MeshPrepareJobImpl;
use renderer_shell_vulkan::VkDeviceContext;
use crate::resource_managers::{PipelineSwapchainInfo, ResourceManager, DescriptorSetAllocatorRef};
use ash::vk;
use crate::pipeline::pipeline::MaterialAsset;
use atelier_assets::loader::handle::Handle;
use crate::pipeline::image::ImageAsset;
use ash::prelude::VkResult;
use crate::resource_managers::DescriptorSetArc;

pub struct MeshExtractJobImpl {
    device_context: VkDeviceContext,
    descriptor_set_allocator: DescriptorSetAllocatorRef,
    pipeline_info: PipelineSwapchainInfo,
    mesh_material: Handle<MaterialAsset>,
    descriptor_set_per_pass: DescriptorSetArc,
    extracted_frame_node_mesh_data: Vec<ExtractedFrameNodeMeshData>,
    extracted_view_node_mesh_data: Vec<ExtractedViewNodeMeshData>,
}

impl MeshExtractJobImpl {
    pub fn new(
        device_context: VkDeviceContext,
        descriptor_set_allocator: DescriptorSetAllocatorRef,
        pipeline_info: PipelineSwapchainInfo,
        mesh_material: &Handle<MaterialAsset>,
        descriptor_set_per_pass: DescriptorSetArc,
    ) -> Self {
        MeshExtractJobImpl {
            device_context,
            descriptor_set_allocator,
            pipeline_info,
            mesh_material: mesh_material.clone(),
            descriptor_set_per_pass,
            extracted_frame_node_mesh_data: Default::default(),
            extracted_view_node_mesh_data: Default::default(),
        }
    }
}

impl DefaultExtractJobImpl<RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext> for MeshExtractJobImpl {
    fn extract_begin(
        &mut self,
        extract_context: &mut RenderJobExtractContext,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) {
        self.extracted_frame_node_mesh_data
            .reserve(frame_packet.frame_node_count(self.feature_index()) as usize);
    }

    fn extract_frame_node(
        &mut self,
        extract_context: &mut RenderJobExtractContext,
        frame_node: PerFrameNode,
        frame_node_index: u32,
    ) {
        let render_node_index = frame_node.render_node_index();
        let render_node_handle = RawSlabKey::<MeshRenderNode>::new(render_node_index);

        let mesh_nodes = extract_context.resources.get::<MeshRenderNodeSet>().unwrap();
        let mesh_render_node = mesh_nodes.meshes.get(render_node_handle).unwrap();

        let position_component = extract_context
            .world
            .get_component::<PositionComponent>(mesh_render_node.entity)
            .unwrap();
        let mesh_component = extract_context
            .world
            .get_component::<MeshComponent>(mesh_render_node.entity)
            .unwrap();

        let mesh_info = extract_context.resource_manager.get_mesh_info(&mesh_component.mesh);

        let draw_calls : Vec<_> = mesh_info.mesh_asset.mesh_parts.iter().map(|mesh_part| {
            let material_instance_info = extract_context.resource_manager.get_material_instance_info(&mesh_part.material_instance);
            let per_material_descriptor = material_instance_info.descriptor_sets[0][1].clone();
            MeshDrawCall {
                vertex_buffer_offset_in_bytes: mesh_part.vertex_buffer_offset_in_bytes,
                vertex_buffer_size_in_bytes: mesh_part.vertex_buffer_size_in_bytes,
                index_buffer_offset_in_bytes: mesh_part.index_buffer_offset_in_bytes,
                index_buffer_size_in_bytes: mesh_part.index_buffer_size_in_bytes,
                per_material_descriptor,
            }
        }).collect();

        let world_transform = glam::Mat4::from_translation(position_component.position);

        self.extracted_frame_node_mesh_data.push(ExtractedFrameNodeMeshData {
            world_transform,
            vertex_buffer: mesh_info.vertex_buffer.clone(),
            index_buffer: mesh_info.index_buffer.clone(),
            draw_calls,
        });
    }

    fn extract_view_node(
        &mut self,
        extract_context: &mut RenderJobExtractContext,
        view: &RenderView,
        view_node: PerViewNode,
        view_node_index: u32,
    ) {
        let frame_node_data = &self.extracted_frame_node_mesh_data[view_node.frame_node_index() as usize];

        let model_view = view.view_matrix() * frame_node_data.world_transform;
        let model_view_proj = view.projection_matrix() * model_view;

        let per_object_param = MeshPerObjectShaderParam {
            model_view,
            model_view_proj
        };

        //TODO: Cache this instead of recalculating it
        let layout = extract_context.resource_manager.get_descriptor_set_info(&self.mesh_material, 0, 2);
        let mut descriptor_set = self.descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(&layout.descriptor_set_layout).unwrap();
        descriptor_set.set_buffer_data(0, &per_object_param);
        descriptor_set.flush(&mut self.descriptor_set_allocator);

        self.extracted_view_node_mesh_data.push(ExtractedViewNodeMeshData {
            per_instance_descriptor: descriptor_set.descriptor_set().clone(),
            frame_node_index: view_node.frame_node_index()
        })
    }

    fn extract_view_finalize(
        &mut self,
        _extract_context: &mut RenderJobExtractContext,
        _view: &RenderView,
    ) {

    }

    fn extract_frame_finalize(
        self,
        _extract_context: &mut RenderJobExtractContext,
    ) -> Box<dyn PrepareJob<RenderJobPrepareContext, RenderJobWriteContext>> {
        let prepare_impl = MeshPrepareJobImpl::new(
            self.device_context,
            self.pipeline_info,
            self.descriptor_set_per_pass,
            self.extracted_frame_node_mesh_data,
            self.extracted_view_node_mesh_data
        );

        Box::new(DefaultPrepareJob::new(prepare_impl))
    }

    fn feature_debug_name(&self) -> &'static str {
        MeshRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        MeshRenderFeature::feature_index()
    }
}
