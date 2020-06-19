use crate::features::mesh::{ExtractedFrameNodeMeshData, MeshRenderNodeSet, MeshRenderFeature, MeshRenderNode, MeshDrawCall, MeshPerObjectShaderParam, ExtractedViewNodeMeshData, MeshPerViewShaderParam};
use crate::{RenderJobExtractContext, PositionComponent, MeshComponent, RenderJobWriteContext, RenderJobPrepareContext, PointLightComponent, SpotLightComponent, DirectionalLightComponent};
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
use legion::prelude::*;

pub struct MeshExtractJobImpl {
    device_context: VkDeviceContext,
    descriptor_set_allocator: DescriptorSetAllocatorRef,
    pipeline_info: PipelineSwapchainInfo,
    mesh_material: Handle<MaterialAsset>,
    descriptor_sets_per_view: Vec<DescriptorSetArc>,
    extracted_frame_node_mesh_data: Vec<Option<ExtractedFrameNodeMeshData>>,
    extracted_view_node_mesh_data: Vec<Vec<Option<ExtractedViewNodeMeshData>>>,
}

impl MeshExtractJobImpl {
    pub fn new(
        device_context: VkDeviceContext,
        descriptor_set_allocator: DescriptorSetAllocatorRef,
        pipeline_info: PipelineSwapchainInfo,
        mesh_material: &Handle<MaterialAsset>,
    ) -> Self {
        MeshExtractJobImpl {
            device_context,
            descriptor_set_allocator,
            pipeline_info,
            mesh_material: mesh_material.clone(),
            descriptor_sets_per_view: Default::default(),
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

        self.extracted_view_node_mesh_data.reserve(views.len());
        for view in views {
            self.extracted_view_node_mesh_data.push(Vec::with_capacity(
                frame_packet.view_node_count(view, self.feature_index()) as usize,
            ));
        }
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
        if mesh_info.is_none() {
            self.extracted_frame_node_mesh_data.push(None);
            return;
        }
        let mesh_info = mesh_info.unwrap();

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

        self.extracted_frame_node_mesh_data.push(Some(ExtractedFrameNodeMeshData {
            world_transform,
            vertex_buffer: mesh_info.vertex_buffer.clone(),
            index_buffer: mesh_info.index_buffer.clone(),
            draw_calls,
        }));
    }

    fn extract_view_node(
        &mut self,
        extract_context: &mut RenderJobExtractContext,
        view: &RenderView,
        view_node: PerViewNode,
        view_node_index: u32,
    ) {
        let frame_node_data = &self.extracted_frame_node_mesh_data[view_node.frame_node_index() as usize];
        if frame_node_data.is_none() {
            self.extracted_view_node_mesh_data[view.view_index() as usize].push(None);
            return;
        }
        let frame_node_data = frame_node_data.as_ref().unwrap();

        let model_view = view.view_matrix() * frame_node_data.world_transform;
        let model_view_proj = view.projection_matrix() * model_view;

        let per_object_param = MeshPerObjectShaderParam {
            model_view,
            model_view_proj
        };

        let layout = extract_context.resource_manager.get_descriptor_set_info(&self.mesh_material, 0, 2);
        let mut descriptor_set = self.descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(&layout.descriptor_set_layout).unwrap();
        descriptor_set.set_buffer_data(0, &per_object_param);
        descriptor_set.flush(&mut self.descriptor_set_allocator);

        self.extracted_view_node_mesh_data[view.view_index() as usize].push(Some(ExtractedViewNodeMeshData {
            per_instance_descriptor: descriptor_set.descriptor_set().clone(),
        }))
    }

    fn extract_view_finalize(
        &mut self,
        extract_context: &mut RenderJobExtractContext,
        view: &RenderView,
    ) {
        let mut per_view_data = MeshPerViewShaderParam::default();

        let query = <(Read<DirectionalLightComponent>)>::query();
        for light in query.iter(&extract_context.world) {
            let light_count = per_view_data.directional_light_count as usize;
            if light_count > per_view_data.directional_lights.len() {
                break;
            }

            let light_from = glam::Vec3::new(0.0, 0.0, 0.0);
            let light_from_vs = (view.view_matrix() * light_from.extend(1.0)).truncate();
            let light_to = light.direction;
            let light_to_vs = (view.view_matrix() * light_to.extend(1.0)).truncate();

            let light_direction = (light_to - light_from).normalize();
            let light_direction_vs = (light_to_vs - light_from_vs).normalize();

            let out = &mut per_view_data.directional_lights[light_count];
            out.direction_ws = light_direction.into();
            out.direction_vs = light_direction_vs.into();
            out.color = light.color;
            out.intensity = light.intensity;

            per_view_data.directional_light_count += 1;
        }

        let query = <(Read<PositionComponent>, Read<PointLightComponent>)>::query();
        for (position, light) in query.iter(&extract_context.world) {
            let light_count = per_view_data.point_light_count as usize;
            if light_count > per_view_data.point_lights.len() {
                break;
            }

            let out = &mut per_view_data.point_lights[light_count];
            out.position_ws = position.position;
            out.position_vs = (view.view_matrix() * position.position.extend(1.0)).truncate();
            out.color = light.color;
            out.range = light.range;
            out.intensity = light.intensity;

            per_view_data.point_light_count += 1;
        }

        let query = <(Read<PositionComponent>, Read<SpotLightComponent>)>::query();
        for (position, light) in query.iter(&extract_context.world) {
            let light_count = per_view_data.spot_light_count as usize;
            if light_count > per_view_data.spot_lights.len() {
                break;
            }

            let light_from = position.position;
            let light_from_vs = (view.view_matrix() * light_from.extend(1.0)).truncate();
            let light_to = position.position + light.direction;
            let light_to_vs = (view.view_matrix() * light_to.extend(1.0)).truncate();

            let light_direction = (light_to - light_from).normalize();
            let light_direction_vs = (light_to_vs - light_from_vs).normalize();

            let out = &mut per_view_data.spot_lights[light_count];
            out.position_ws = light_from.into();
            out.position_vs = light_from_vs.into();
            out.direction_ws = light_direction.into();
            out.direction_vs = light_direction_vs.into();
            out.spotlight_half_angle = light.spotlight_half_angle;
            out.color = light.color;
            out.range = light.range;
            out.intensity = light.intensity;

            per_view_data.spot_light_count += 1;
        }

        //TODO: We should probably set these up per view (so we can pick the best lights based on
        // the view)
        let layout = extract_context.resource_manager.get_descriptor_set_info(&self.mesh_material, 0, 0);
        let mut descriptor_set = self.descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(&layout.descriptor_set_layout).unwrap();
        descriptor_set.set_buffer_data(0, &per_view_data);
        descriptor_set.flush(&mut self.descriptor_set_allocator);

        self.descriptor_sets_per_view.push(descriptor_set.descriptor_set().clone());
    }

    fn extract_frame_finalize(
        self,
        _extract_context: &mut RenderJobExtractContext,
    ) -> Box<dyn PrepareJob<RenderJobPrepareContext, RenderJobWriteContext>> {
        let prepare_impl = MeshPrepareJobImpl::new(
            self.device_context,
            self.pipeline_info,
            self.descriptor_sets_per_view,
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
