use std::sync::Arc;

use anyhow::Result;
use gpu_allocator::MemoryLocation;
use instanced::InstancedRenderer;
use nalgebra::Vector2;
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

use render_description::{RenderDescription, RenderPipelineDescription};

mod instanced;

pub mod render_description;

pub(crate) struct SharedGpuResources {
    pub(crate) uniform_buffer_global: Buffer,
    pub(crate) image_depth: Image,
}

pub struct Renderer {
    device: Arc<Device>,

    shared_gpu_resources: SharedGpuResources,
    scene_uniform_data: Vec<u8>,

    graphics_pipelines: Vec<Pipeline>,
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
            size: render_description.scene_uniform_data_size,
            usage_flags: vk::BufferUsageFlags::UNIFORM_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        })?;
        let image_depth = Self::create_image_depth(&device)?;

        let shared_gpu_resources = SharedGpuResources {
            uniform_buffer_global,
            image_depth,
        };
        let scene_uniform_data =
            Vec::with_capacity(render_description.scene_uniform_data_size as _);

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
                    &device,
                    draw_data,
                    descriptor_set_layouts[0].clone(),
                    &shared_gpu_resources,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        let mut renderers_grouped_by_pipeline = vec![vec![]; graphics_pipelines.len()];
        for (index, renderer) in instanced_renderers.iter().enumerate() {
            renderers_grouped_by_pipeline[renderer.draw_data.pipeline_index].push(index);
        }

        Ok(Self {
            device,
            shared_gpu_resources,
            scene_uniform_data,
            instanced_renderers,
            graphics_pipelines,
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

        let (vertex_input_attributes, vertex_input_bindings) = (
            vec![vk::VertexInputAttributeDescription::default()
                .location(0)
                .binding(0)
                .format(vk::Format::R32G32B32_SFLOAT)],
            vec![vk::VertexInputBindingDescription::default()
                .binding(0)
                .stride(12)
                .input_rate(vk::VertexInputRate::VERTEX)],
        );

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
        let layout_descriptor = DescriptorSetLayoutDescriptor {
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
            device.create_descriptor_set_layout(layout_descriptor)?;

        Ok(vec![Arc::new(instanced_single_vertices_layout)])
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

    pub fn update_scene_uniform_data(&mut self, data: &[u8]) {
        self.scene_uniform_data.resize(data.len(), 0);
        self.scene_uniform_data[..data.len()].copy_from_slice(data);
    }

    pub fn render(&mut self) -> Result<()> {
        self.device.frame_begin()?;

        // XXX FIXME: may be dangerous as uniform buffer may still be read in current frame's draws
        //            as we are writing to it.
        self.shared_gpu_resources
            .uniform_buffer_global
            .write_data(&self.scene_uniform_data)?;

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
                self.instanced_renderers[*renderer_index].record_commands(
                    &command_buffer,
                    &self.graphics_pipelines[pipeline_index],
                    self.device.current_frame(),
                )?;
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
