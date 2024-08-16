use std::{mem::size_of, sync::Arc};

use anyhow::Result;
use gpu_allocator::MemoryLocation;
use instanced::InstancedRenderer;
use nalgebra::{Matrix4, Vector2, Vector3};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use crate::vulkan::{
    device::Device,
    resource::{
        Buffer, BufferDescriptor, DescriptorSetLayout, DescriptorSetLayoutDescriptor, Image,
        ImageDescriptor, Pipeline, PipelineDescriptor,
    },
    types::{DescriptorSetLayoutBinding, PipelineDepthStencilState, PipelineRasterizationState},
    vk,
};

use render_description::{
    InstancedDrawData, InstancedType, RenderDescription, RenderPipelineDescription,
};

mod instanced;

pub mod render_description;

pub(crate) struct InstancedSingleVerticesGpuResources {
    vertex_buffer_positions: Buffer,
    index_buffer: Buffer,
    storage_buffer_instances: Buffer,
}

pub(crate) struct InstancedDynamicVerticesGpuResources {
    storage_buffer_vertex_positions: Buffer,
    storage_buffer_indices: Buffer,
    storage_buffer_instances: Buffer,
}

pub(crate) enum InstancedGpuResources {
    SingleVertices(InstancedSingleVerticesGpuResources),
    DynamicVertices(InstancedDynamicVerticesGpuResources),
}

pub(crate) fn create_instanced_gpu_resources(
    device: &Arc<Device>,
    draw_data: &InstancedDrawData,
) -> Result<InstancedGpuResources> {
    // XXX TODO: Make all buffers reside in GPU memory only.

    let storage_buffer_instances = device.create_buffer(BufferDescriptor {
        size: (draw_data.instance_data.len() * size_of::<u8>()) as u64,
        usage_flags: vk::BufferUsageFlags::STORAGE_BUFFER,
        memory_location: MemoryLocation::CpuToGpu,
    })?;
    storage_buffer_instances.write_data(&draw_data.instance_data)?;

    let gpu_resources = match draw_data.instanced_type {
        InstancedType::SingleVertices => {
            let vertex_buffer_positions = device.create_buffer(BufferDescriptor {
                size: (draw_data.vertices.len() * size_of::<Vector3<f32>>()) as u64,
                usage_flags: vk::BufferUsageFlags::VERTEX_BUFFER,
                memory_location: MemoryLocation::CpuToGpu,
            })?;
            vertex_buffer_positions.write_data(&draw_data.vertices)?;

            let index_buffer = device.create_buffer(BufferDescriptor {
                size: (draw_data.indices.len() * size_of::<u16>()) as u64,
                usage_flags: vk::BufferUsageFlags::INDEX_BUFFER,
                memory_location: MemoryLocation::CpuToGpu,
            })?;
            index_buffer.write_data(&draw_data.indices)?;

            InstancedGpuResources::SingleVertices(InstancedSingleVerticesGpuResources {
                vertex_buffer_positions,
                index_buffer,
                storage_buffer_instances,
            })
        }
        InstancedType::DynamicVertices => {
            let storage_buffer_vertex_positions = device.create_buffer(BufferDescriptor {
                size: (draw_data.vertices.len() * size_of::<Vector3<f32>>()) as u64,
                usage_flags: vk::BufferUsageFlags::STORAGE_BUFFER,
                memory_location: MemoryLocation::CpuToGpu,
            })?;
            storage_buffer_vertex_positions.write_data(&draw_data.vertices)?;

            let storage_buffer_indices = device.create_buffer(BufferDescriptor {
                size: (draw_data.indices.len() * size_of::<u16>()) as u64,
                usage_flags: vk::BufferUsageFlags::STORAGE_BUFFER,
                memory_location: MemoryLocation::CpuToGpu,
            })?;
            storage_buffer_indices.write_data(&draw_data.indices)?;

            InstancedGpuResources::DynamicVertices(InstancedDynamicVerticesGpuResources {
                storage_buffer_vertex_positions,
                storage_buffer_indices,
                storage_buffer_instances,
            })
        }
    };

    Ok(gpu_resources)
}

pub(crate) struct SharedGpuResources {
    pub(crate) uniform_buffer_global: Buffer,
    pub(crate) image_depth: Image,
}

#[derive(Clone, Copy, Default)]
pub struct GlobalUniformBufferGpuData {
    pub view_projection: Matrix4<f32>,
    pub runner_transform: Matrix4<f32>,
}

const INSTANCED_SINGLE_VERTICES_DESCRIPTOR_LAYOUT_INDEX: usize = 0;
const INSTANCED_DYNAMIC_VERTICES_DESCRIPTOR_LAYOUT_INDEX: usize = 1;

pub struct Renderer {
    device: Arc<Device>,

    uniform_buffer_global_data: GlobalUniformBufferGpuData,
    shared_gpu_resources: SharedGpuResources,

    graphics_pipelines: Vec<Pipeline>,
    descriptor_set_layouts: Vec<Arc<DescriptorSetLayout>>,

    instanced_renderers: Vec<InstancedRenderer>,

    /// Contains index to `instanced_renderers` Vec, grouped by index to `graphics_pipeline` Vec.
    renderers_grouped_by_pipeline: Vec<Vec<usize>>,
}

impl Renderer {
    pub fn new(
        window_handle: RawWindowHandle,
        display_handle: RawDisplayHandle,
        render_description: RenderDescription,
    ) -> Result<Self> {
        let device = Arc::new(Device::new(window_handle, display_handle)?);

        let uniform_buffer_global = device.create_buffer(BufferDescriptor {
            size: size_of::<GlobalUniformBufferGpuData>() as _,
            usage_flags: vk::BufferUsageFlags::UNIFORM_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        })?;
        let image_depth = Self::create_image_depth(&device)?;

        let shared_gpu_resources = SharedGpuResources {
            uniform_buffer_global,
            image_depth,
        };

        let descriptor_set_layouts = Self::create_descriptor_set_layouts(&device)?;
        let graphics_pipelines = render_description
            .pipelines
            .into_iter()
            .map(|p| {
                Self::create_graphics_pipeline(
                    &device,
                    descriptor_set_layouts[0].clone(),
                    shared_gpu_resources.image_depth.format,
                    p,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        let instanced_renderers = render_description
            .instanced_draw_data
            .into_iter()
            .map(|draw_data| {
                InstancedRenderer::new(
                    device.clone(),
                    draw_data,
                    descriptor_set_layouts[0].clone(),
                    &shared_gpu_resources,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        let mut renderers_grouped_by_pipeline: Vec<Vec<usize>> =
            Vec::with_capacity(graphics_pipelines.len());
        for (index, renderer) in instanced_renderers.iter().enumerate() {
            renderers_grouped_by_pipeline[renderer.draw_data.pipeline_index].push(index);
        }

        Ok(Self {
            device,
            uniform_buffer_global_data: GlobalUniformBufferGpuData::default(),
            shared_gpu_resources,
            instanced_renderers,
            graphics_pipelines,
            descriptor_set_layouts,
            renderers_grouped_by_pipeline,
        })
    }

    fn create_graphics_pipeline(
        device: &Arc<Device>,
        descriptor_set_layout: Arc<DescriptorSetLayout>,
        depth_attachment_format: vk::Format,
        render_pipeline_description: RenderPipelineDescription,
    ) -> Result<Pipeline> {
        let shader_modules = render_pipeline_description
            .shader_modules
            .into_iter()
            .map(|d| device.create_shader_module(d))
            .collect::<Result<Vec<_>>>()?;

        let (vertex_input_attributes, vertex_input_bindings) =
            if let InstancedType::SingleVertices = render_pipeline_description.instanced_type {
                (
                    vec![vk::VertexInputAttributeDescription::default()
                        .location(0)
                        .binding(0)
                        .format(vk::Format::R32G32B32_SFLOAT)],
                    vec![vk::VertexInputBindingDescription::default()
                        .binding(0)
                        .stride(12)
                        .input_rate(vk::VertexInputRate::VERTEX)],
                )
            } else {
                (vec![], vec![])
            };

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
            shader_modules,
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

    fn create_descriptor_set_layouts(device: &Device) -> Result<Vec<Arc<DescriptorSetLayout>>> {
        let instanced_single_vertices_layout_descriptor = DescriptorSetLayoutDescriptor {
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
        let instanced_single_vertices_layout =
            device.create_descriptor_set_layout(instanced_single_vertices_layout_descriptor)?;

        // XXX TODO: Can just completely get rid of this since we can control the vertex and index offset.
        //           The whole platform is just one big mesh.
        let instanced_dynamic_vertices_layout_descriptor = DescriptorSetLayoutDescriptor {
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
                DescriptorSetLayoutBinding::new()
                    .binding(2)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .stage_flags(vk::ShaderStageFlags::VERTEX),
                DescriptorSetLayoutBinding::new()
                    .binding(3)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .stage_flags(vk::ShaderStageFlags::VERTEX),
            ],
            flags: vk::DescriptorSetLayoutCreateFlags::empty(),
            binding_flags: None,
        };
        let instanced_dynamic_vertices_layout =
            device.create_descriptor_set_layout(instanced_dynamic_vertices_layout_descriptor)?;

        Ok(vec![
            Arc::new(instanced_single_vertices_layout),
            Arc::new(instanced_dynamic_vertices_layout),
        ])
    }

    fn create_image_depth(device: &Arc<Device>) -> Result<Image> {
        let image_depth_desc = ImageDescriptor {
            width: device.swapchain_extent().width,
            height: device.swapchain_extent().height,
            depth: 1,

            array_layer_count: 1,
            mip_level_count: 1,

            format: vk::Format::D32_SFLOAT,
            image_type: vk::ImageType::TYPE_2D,
            usage_flags: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,

            memory_location: MemoryLocation::GpuOnly,
        };
        device.create_image(image_depth_desc)
    }

    pub fn update_scene_constants(&mut self, scene_constants: GlobalUniformBufferGpuData) {
        self.uniform_buffer_global_data = scene_constants;
    }

    pub fn render(&mut self) -> Result<()> {
        self.device.frame_begin()?;

        // XXX FIXME: may be dangerous as uniform buffer may still be read in current frame's draws
        //            as we are writing to it.
        self.shared_gpu_resources
            .uniform_buffer_global
            .write_data(std::slice::from_ref(&self.uniform_buffer_global_data))?;

        let command_buffer = self.device.get_current_command_buffer()?;

        command_buffer.begin()?;
        self.device
            .command_transition_swapchain_image_layout_to_color_attachment(&command_buffer);
        self.device.command_begin_rendering_swapchain(
            &command_buffer,
            [1.0, 1.0, 1.0, 1.0],
            Some(&self.shared_gpu_resources.image_depth),
        );

        for (pipeline_index, renderers) in self.renderers_grouped_by_pipeline.iter().enumerate() {
            command_buffer.bind_pipeline_graphics(&self.graphics_pipelines[pipeline_index]);
            for renderer_index in renderers {
                self.instanced_renderers[*renderer_index]
                    .record_commands(&command_buffer, &self.graphics_pipelines[pipeline_index])?;
            }
        }

        command_buffer.end_rendering();
        self.device
            .command_transition_swapchain_image_layout_to_present(&command_buffer);
        command_buffer.end()?;

        self.device.queue_submit_commands_graphics(command_buffer)?;
        self.device.swapchain_present()?;

        Ok(())
    }

    pub fn swapchain_extent(&self) -> Vector2<u32> {
        Vector2::new(
            self.device.swapchain_extent().width,
            self.device.swapchain_extent().height,
        )
    }
}
