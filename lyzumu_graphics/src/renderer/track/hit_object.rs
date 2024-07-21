use std::{mem::size_of, sync::Arc};

use anyhow::Result;
use nalgebra::{Matrix4, Vector3, Vector4};

use crate::{
    scene::{SceneHitObject, TrackScene},
    vulkan::{
        command::CommandBuffer,
        device::Device,
        gpu_allocator::MemoryLocation,
        resource::{
            Buffer, BufferDescriptor, DescriptorBindingBufferWrite, DescriptorBindingWrites,
            DescriptorSet, DescriptorSetDescriptor, DescriptorSetLayout,
            DescriptorSetLayoutDescriptor, DescriptorSetPoolType, Image, Pipeline,
            PipelineDescriptor,
        },
        shader::{ShaderModuleDescriptor, ShaderStage},
        types::{
            DescriptorSetLayoutBinding, PipelineDepthStencilState, PipelineRasterizationState,
        },
        vk,
    },
};

use super::HitObjectSharedContext;

#[derive(Clone, Copy)]
struct InstanceData {
    transform: Matrix4<f32>,
    base_color: Vector4<f32>,
}

impl From<SceneHitObject> for InstanceData {
    fn from(scene_hit_object: SceneHitObject) -> Self {
        InstanceData {
            transform: scene_hit_object.transform,
            base_color: scene_hit_object.base_color,
        }
    }
}

/// Required GPU resources for a specific track.
struct HitObjectSceneData {
    current_scene: TrackScene,
    vertex_buffer_positions: Buffer,
    index_buffer: Buffer,
    storage_buffer_instances: Buffer,
}

pub(crate) struct HitObjectRenderer {
    scene_data: Option<HitObjectSceneData>,

    descriptor_set: DescriptorSet,
    graphics_pipeline: Pipeline,

    device: Arc<Device>,
    shared: Arc<HitObjectSharedContext>,
}

impl HitObjectRenderer {
    pub(crate) fn new(device: Arc<Device>, shared: Arc<HitObjectSharedContext>) -> Result<Self> {
        let descriptor_set_layout = Arc::new(Self::create_descriptor_set_layout(&device)?);
        let descriptor_set = device.create_descriptor_set(DescriptorSetDescriptor {
            layout: descriptor_set_layout.clone(),
            pool_type: DescriptorSetPoolType::GlobalGenericResource,
        })?;
        let graphics_pipeline = Self::create_graphics_pipeline(
            &device,
            descriptor_set_layout,
            shared.image_depth.format,
        )?;

        Ok(Self {
            device,
            shared,
            descriptor_set,
            graphics_pipeline,
            scene_data: None,
        })
    }

    pub(crate) fn write_render_commands(
        &self,
        command_buffer: &CommandBuffer,
        _current_frame: u64,
    ) -> Result<()> {
        let scene_data = self
            .scene_data
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Scene data is not loaded!"))?;

        command_buffer.bind_pipeline_graphics(&self.graphics_pipeline);
        command_buffer.bind_descriptor_set_graphics(&self.descriptor_set, &self.graphics_pipeline);

        command_buffer.bind_vertex_buffers(0, &[&scene_data.vertex_buffer_positions], &[0]);
        command_buffer.bind_index_buffer(&scene_data.index_buffer, 0);
        command_buffer.draw_indexed(
            scene_data.current_scene.hit_object_mesh.indices.len() as _,
            scene_data.current_scene.hit_objects.len() as _,
            0,
            0,
            0,
        );

        Ok(())
    }

    pub(crate) fn load_scene(&mut self, scene: TrackScene) -> Result<()> {
        // Create buffers.
        // XXX: These should be GPU only memory.
        let vertex_buffer_positions = self.device.create_buffer(BufferDescriptor {
            size: (scene.hit_object_mesh.vertices.len() * size_of::<Vector3<f32>>()) as u64,
            usage_flags: vk::BufferUsageFlags::VERTEX_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        })?;
        let index_buffer = self.device.create_buffer(BufferDescriptor {
            size: (scene.hit_object_mesh.indices.len() * size_of::<u16>()) as u64,
            usage_flags: vk::BufferUsageFlags::INDEX_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        })?;
        let storage_buffer_instances = self.device.create_buffer(BufferDescriptor {
            size: (scene.hit_objects.len() * size_of::<InstanceData>()) as u64,
            usage_flags: vk::BufferUsageFlags::STORAGE_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        })?;

        // Upload data to buffer.
        vertex_buffer_positions.write_data(&scene.hit_object_mesh.vertices)?;
        index_buffer.write_data(&scene.hit_object_mesh.indices)?;
        let instance_data = scene
            .hit_objects
            .clone()
            .into_iter()
            .map(InstanceData::from)
            .collect::<Vec<_>>();
        storage_buffer_instances.write_data(&instance_data)?;

        // Update descriptor set(s).
        let descriptor_binding_writes = DescriptorBindingWrites {
            buffers: vec![
                DescriptorBindingBufferWrite {
                    buffer: &self.shared.uniform_buffer_global,
                    binding_index: 0,
                },
                DescriptorBindingBufferWrite {
                    buffer: &storage_buffer_instances,
                    binding_index: 1,
                },
            ],
        };
        self.device
            .update_descriptor_set(&self.descriptor_set, &descriptor_binding_writes)?;

        self.scene_data = Some(HitObjectSceneData {
            current_scene: scene,
            vertex_buffer_positions,
            index_buffer,
            storage_buffer_instances,
        });
        Ok(())
    }

    fn create_descriptor_set_layout(device: &Device) -> Result<DescriptorSetLayout> {
        let descriptor = DescriptorSetLayoutDescriptor {
            bindings: vec![
                DescriptorSetLayoutBinding::new()
                    .binding(0)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .stage_flags(vk::ShaderStageFlags::VERTEX),
                DescriptorSetLayoutBinding::new()
                    .binding(1)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .stage_flags(vk::ShaderStageFlags::VERTEX),
            ],
            flags: vk::DescriptorSetLayoutCreateFlags::empty(),
            binding_flags: None,
        };

        device.create_descriptor_set_layout(descriptor)
    }

    fn create_graphics_pipeline(
        device: &Arc<Device>,
        descriptor_set_layout: Arc<DescriptorSetLayout>,
        depth_attachment_format: vk::Format,
    ) -> Result<Pipeline> {
        let vertex_shader_module = device.create_shader_module(ShaderModuleDescriptor {
            source_file_name: "shaders/hit_object.vs.glsl",
            shader_stage: ShaderStage::Vertex,
        })?;
        let fragment_shader_module = device.create_shader_module(ShaderModuleDescriptor {
            source_file_name: "shaders/hit_object.fs.glsl",
            shader_stage: ShaderStage::Fragment,
        })?;

        let vertex_input_attributes = vec![vk::VertexInputAttributeDescription::default()
            .location(0)
            .binding(0)
            .format(vk::Format::R32G32B32_SFLOAT)];
        let vertex_input_bindings = vec![vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(12)
            .input_rate(vk::VertexInputRate::VERTEX)];

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA);

        let rasterization_state = PipelineRasterizationState::new()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::empty());

        let depth_stencil_state = PipelineDepthStencilState::new()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
            .min_depth_bounds(0.0)
            .max_depth_bounds(1.0);

        let pipeline_descriptor = PipelineDescriptor {
            descriptor_set_layouts: vec![descriptor_set_layout],
            shader_modules: vec![vertex_shader_module, fragment_shader_module],
            vertex_input_attributes,
            vertex_input_bindings,
            viewport_scissor_extent: device.swapchain_extent(),
            primitive_topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            color_blend_attachments: vec![color_blend_attachment],
            depth_stencil_state,
            rasterization_state,
            color_attachment_formats: vec![device.swapchain_color_format()],
            depth_attachment_format,
        };

        device.create_pipeline(pipeline_descriptor)
    }
}
