use std::{mem::size_of, sync::Arc};

use anyhow::Result;
use nalgebra::{Matrix4, Vector3, Vector4};

use crate::vulkan::{
    command::CommandBuffer,
    device::Device,
    gpu_allocator::MemoryLocation,
    resource::{
        Buffer, BufferDescriptor, DescriptorBindingBufferWrite, DescriptorBindingWrites,
        DescriptorSet, DescriptorSetDescriptor, DescriptorSetLayout, DescriptorSetLayoutDescriptor,
        DescriptorSetPoolType, Image, Pipeline, PipelineDescriptor,
    },
    shader::{ShaderModuleDescriptor, ShaderStage},
    types::{DescriptorSetLayoutBinding, PipelineDepthStencilState, PipelineRasterizationState},
    vk,
};

use super::{
    create_instanced_gpu_resources, render_description::InstancedDrawData, InstancedGpuResources,
    SharedGpuResources,
};

// use super::HitObjectSharedContext;

/// Required GPU resources for a specific track.
// struct HitObjectSceneData {
//     current_scene: TrackScene,
//     vertex_buffer_positions: Buffer,
//     index_buffer: Buffer,
//     storage_buffer_instances: Buffer,
// }
//
// struct HitBarResources {
//     vertex_buffer_positions: Buffer,
//     index_buffer: Buffer,
//     mesh: Mesh,
// }

// impl HitBarResources {
//     fn new(device: &Arc<Device>) -> Result<Self> {
//         let mesh = Mesh::from(Polyhedron::cuboid(2.0, 0.05, 0.2));
//
//         let vertex_buffer_positions = device.create_buffer(BufferDescriptor {
//             size: (mesh.vertices.len() * size_of::<Vector3<f32>>()) as u64,
//             usage_flags: vk::BufferUsageFlags::VERTEX_BUFFER,
//             memory_location: MemoryLocation::CpuToGpu,
//         })?;
//         vertex_buffer_positions.write_data(&mesh.vertices)?;
//         let index_buffer = device.create_buffer(BufferDescriptor {
//             size: (mesh.indices.len() * size_of::<u16>()) as u64,
//             usage_flags: vk::BufferUsageFlags::INDEX_BUFFER,
//             memory_location: MemoryLocation::CpuToGpu,
//         })?;
//         index_buffer.write_data(&mesh.indices)?;
//
//         Ok(Self {
//             vertex_buffer_positions,
//             index_buffer,
//             mesh,
//         })
//     }
// }

pub(crate) struct InstancedRenderer {
    pub(crate) draw_data: InstancedDrawData,
    gpu_resources: InstancedGpuResources,
    descriptor_set: DescriptorSet,
    device: Arc<Device>,
}

impl InstancedRenderer {
    pub(crate) fn new(
        device: Arc<Device>,
        draw_data: InstancedDrawData,
        descriptor_set_layout: Arc<DescriptorSetLayout>,
        shared_resources: &SharedGpuResources,
    ) -> Result<Self> {
        let descriptor_set = device.create_descriptor_set(DescriptorSetDescriptor {
            layout: descriptor_set_layout.clone(),
            pool_type: DescriptorSetPoolType::GlobalGenericResource,
        })?;
        // let graphics_pipeline = Self::create_graphics_pipeline(
        //     &device,
        //     descriptor_set_layout,
        //     shared.image_depth.format,
        // )?;
        //
        // let hit_bar_resources = HitBarResources::new(&device)?;
        let gpu_resources = create_instanced_gpu_resources(&device, &draw_data)?;

        Self::update_descriptor_set(&device, &descriptor_set, &gpu_resources, shared_resources)?;

        Ok(Self {
            draw_data,
            gpu_resources,
            descriptor_set,
            device,
            // shared,
            // descriptor_set,
            // graphics_pipeline,
            // scene_data: None,
            // hit_bar_resources,
        })
    }

    pub(crate) fn record_commands(
        &self,
        command_buffer: &CommandBuffer,
        graphics_pipeline: &Pipeline,
        // _current_frame: u64,
    ) -> Result<()> {
        // let scene_data = self
        //     .scene_data
        //     .as_ref()
        //     .ok_or_else(|| anyhow::anyhow!("Scene data is not loaded!"))?;
        //
        // command_buffer.bind_pipeline_graphics(&self.graphics_pipeline);
        command_buffer.bind_descriptor_set_graphics(&self.descriptor_set, &graphics_pipeline);

        if let InstancedGpuResources::SingleVertices(resources) = &self.gpu_resources {
            command_buffer.bind_vertex_buffers(0, &[&resources.vertex_buffer_positions], &[0]);
            command_buffer.bind_index_buffer(&resources.index_buffer, 0);
        }

        // XXX TODO: Only draw visible objects based on audio position.
        command_buffer.draw_indexed(
            self.draw_data.indices.len() as _,
            self.draw_data.instance_count as _,
            0,
            0,
            0,
        );

        // XXX: Draw hit bar. Instance is hacked into last element in SSBO.
        // command_buffer.bind_vertex_buffers(
        //     0,
        //     &[&self.hit_bar_resources.vertex_buffer_positions],
        //     &[0],
        // );
        // command_buffer.bind_index_buffer(&self.hit_bar_resources.index_buffer, 0);
        // command_buffer.draw_indexed(
        //     self.hit_bar_resources.mesh.indices.len() as _,
        //     1,
        //     0,
        //     0,
        //     scene_data.current_scene.hit_objects.len() as _,
        // );

        Ok(())
    }

    pub(crate) fn update_descriptor_set(
        device: &Device,
        descriptor_set: &DescriptorSet,
        instanced_resources: &InstancedGpuResources,
        shared_resources: &SharedGpuResources,
    ) -> Result<()> {
        let storage_buffer_instances = match instanced_resources {
            InstancedGpuResources::SingleVertices(resources) => &resources.storage_buffer_instances,
            InstancedGpuResources::DynamicVertices(resources) => {
                &resources.storage_buffer_instances
            }
        };

        // XXX TODO: Make binding layouts configurable by the user.
        let mut descriptor_binding_writes = DescriptorBindingWrites {
            buffers: vec![
                DescriptorBindingBufferWrite {
                    buffer: &shared_resources.uniform_buffer_global,
                    binding_index: 0,
                },
                DescriptorBindingBufferWrite {
                    buffer: storage_buffer_instances,
                    binding_index: 1,
                },
            ],
        };

        if let InstancedGpuResources::DynamicVertices(resources) = instanced_resources {
            descriptor_binding_writes.buffers.extend(vec![
                DescriptorBindingBufferWrite {
                    buffer: &resources.storage_buffer_vertex_positions,
                    binding_index: 2,
                },
                DescriptorBindingBufferWrite {
                    buffer: &resources.storage_buffer_indices,
                    binding_index: 3,
                },
            ]);
        };

        device.update_descriptor_set(descriptor_set, &descriptor_binding_writes)?;

        Ok(())
    }

    // fn create_descriptor_set_layout(device: &Device) -> Result<DescriptorSetLayout> {
    //     let descriptor = DescriptorSetLayoutDescriptor {
    //         bindings: vec![
    //             DescriptorSetLayoutBinding::new()
    //                 .binding(0)
    //                 .descriptor_count(1)
    //                 .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
    //                 .stage_flags(vk::ShaderStageFlags::VERTEX),
    //             DescriptorSetLayoutBinding::new()
    //                 .binding(1)
    //                 .descriptor_count(1)
    //                 .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
    //                 .stage_flags(vk::ShaderStageFlags::VERTEX),
    //         ],
    //         flags: vk::DescriptorSetLayoutCreateFlags::empty(),
    //         binding_flags: None,
    //     };
    //
    //     device.create_descriptor_set_layout(descriptor)
    // }

    // fn create_graphics_pipeline(
    //     device: &Arc<Device>,
    //     descriptor_set_layout: Arc<DescriptorSetLayout>,
    //     depth_attachment_format: vk::Format,
    // ) -> Result<Pipeline> {
    //     let vertex_shader_module = device.create_shader_module(ShaderModuleDescriptor {
    //         source_file_name: "shaders/hit_object.vs.glsl",
    //         shader_stage: ShaderStage::Vertex,
    //     })?;
    //     let fragment_shader_module = device.create_shader_module(ShaderModuleDescriptor {
    //         source_file_name: "shaders/hit_object.fs.glsl",
    //         shader_stage: ShaderStage::Fragment,
    //     })?;
    //
    //     let vertex_input_attributes = vec![vk::VertexInputAttributeDescription::default()
    //         .location(0)
    //         .binding(0)
    //         .format(vk::Format::R32G32B32_SFLOAT)];
    //     let vertex_input_bindings = vec![vk::VertexInputBindingDescription::default()
    //         .binding(0)
    //         .stride(12)
    //         .input_rate(vk::VertexInputRate::VERTEX)];
    //
    //     let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
    //         .blend_enable(false)
    //         .color_write_mask(vk::ColorComponentFlags::RGBA);
    //
    //     let rasterization_state = PipelineRasterizationState::new()
    //         .polygon_mode(vk::PolygonMode::FILL)
    //         .cull_mode(vk::CullModeFlags::empty());
    //
    //     let depth_stencil_state = PipelineDepthStencilState::new()
    //         .depth_test_enable(true)
    //         .depth_write_enable(true)
    //         .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
    //         .min_depth_bounds(0.0)
    //         .max_depth_bounds(1.0);
    //
    //     let pipeline_descriptor = PipelineDescriptor {
    //         descriptor_set_layouts: vec![descriptor_set_layout],
    //         shader_modules: vec![vertex_shader_module, fragment_shader_module],
    //         vertex_input_attributes,
    //         vertex_input_bindings,
    //         viewport_scissor_extent: device.swapchain_extent(),
    //         primitive_topology: vk::PrimitiveTopology::TRIANGLE_LIST,
    //         color_blend_attachments: vec![color_blend_attachment],
    //         depth_stencil_state,
    //         rasterization_state,
    //         color_attachment_formats: vec![device.swapchain_color_format()],
    //         depth_attachment_format,
    //     };
    //
    //     device.create_pipeline(pipeline_descriptor)
    // }
}
