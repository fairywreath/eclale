use std::sync::Arc;

use anyhow::Result;
use ash::vk;
use gpu_allocator::MemoryLocation;

use crate::{
    resource::{BufferDescriptor, Image},
    QUEUE_FAMILY_INDEX_GRAPHICS,
};

use super::{
    device::Device,
    resource::{Buffer, DescriptorSet, Pipeline},
    DeviceShared,
};

/// Structure that wraps around the raw vulkan CommandPool object.
pub(crate) struct CommandPool {
    pub(crate) raw: vk::CommandPool,
    device: Arc<DeviceShared>,
}

impl CommandPool {
    pub(crate) fn new(device: Arc<DeviceShared>, queue_family_index: u32) -> Result<Self> {
        let command_pool_info =
            vk::CommandPoolCreateInfo::default().queue_family_index(queue_family_index);

        let raw = unsafe {
            let command_pool = device.raw.create_command_pool(&command_pool_info, None)?;
            device
                .raw
                .reset_command_pool(command_pool, vk::CommandPoolResetFlags::empty())?;
            command_pool
        };

        Ok(Self { raw, device })
    }

    pub(crate) fn allocate_command_buffers(
        &self,
        level: vk::CommandBufferLevel,
        count: u32,
    ) -> Result<Vec<vk::CommandBuffer>> {
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.raw)
            .level(level)
            .command_buffer_count(count);
        let command_buffers = unsafe { self.device.raw.allocate_command_buffers(&allocate_info)? };
        Ok(command_buffers)
    }

    pub(crate) fn reset(&self) -> Result<()> {
        unsafe {
            self.device
                .raw
                .reset_command_pool(self.raw, vk::CommandPoolResetFlags::empty())?
        }
        Ok(())
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe { self.device.raw.destroy_command_pool(self.raw, None) }
    }
}

/// Handles command buffer creation and usage. Properly manages per-pool/frame/thread command resources.
pub(crate) struct CommandBufferManager {
    _device: Arc<DeviceShared>,
    command_pools: Vec<CommandPool>,
    command_buffers: Vec<CommandBuffer>,
    num_command_buffers_per_pool: u32,
    num_used_command_buffers_per_pool: Vec<u32>,
}

impl CommandBufferManager {
    /// Creates a manager instance and creates all required GPU command resources.
    pub(crate) fn new(
        device: Arc<DeviceShared>,
        num_command_pools: u32,
        num_command_buffers_per_pool: u32,
    ) -> Result<Self> {
        let command_pools = (0..num_command_pools)
            .map(|_| {
                Ok(CommandPool::new(
                    device.clone(),
                    device.queue_families[QUEUE_FAMILY_INDEX_GRAPHICS].index,
                )?)
            })
            .collect::<Result<Vec<_>>>()?;

        let command_buffers = (0..num_command_pools)
            .map(|pool_index| {
                Ok(command_pools[pool_index as usize]
                    .allocate_command_buffers(
                        vk::CommandBufferLevel::PRIMARY,
                        num_command_buffers_per_pool,
                    )?
                    .into_iter()
                    .map(|raw| CommandBuffer::new_from_vulkan_handle(raw, device.clone()))
                    .collect::<Vec<_>>())
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        Ok(Self {
            _device: device,
            command_pools,
            command_buffers,
            num_command_buffers_per_pool,
            num_used_command_buffers_per_pool: vec![0; num_command_pools as _],
        })
    }

    pub(crate) fn reset_command_pools(&mut self, pool_indices: &[usize]) -> Result<()> {
        for &pool_index in pool_indices {
            self.command_pools[pool_index].reset()?;
            self.num_used_command_buffers_per_pool[pool_index] = 0;
        }

        Ok(())
    }

    pub(crate) fn get_command_buffer_at_pool(
        &mut self,
        pool_index: usize,
    ) -> Result<CommandBuffer> {
        let num_used_buffers = self.num_used_command_buffers_per_pool[pool_index as usize];
        if num_used_buffers > self.num_command_buffers_per_pool {
            return Err(anyhow::anyhow!(
                "All command buffers in current frame thread are already used!"
            ));
        }
        self.num_used_command_buffers_per_pool[pool_index as usize] += 1;

        let index =
            (pool_index * self.num_command_buffers_per_pool as usize) + num_used_buffers as usize;
        Ok(self.command_buffers[index].clone())
    }
}

/// Do not need to hold the command pool resource here. Command pools is held by the 'Device' structure which handles all
/// GPU rendering logic. When 'Device' is dropped all command pools are destroyed, hence this command buffer will be invalid,
/// but we also lose the ability so actually submit command buffers hence the command buffers are no longer required.
#[derive(Clone)]
pub struct CommandBuffer {
    pub(crate) raw: vk::CommandBuffer,
    device: Arc<DeviceShared>,
}

impl CommandBuffer {
    fn new_from_vulkan_handle(raw: vk::CommandBuffer, device: Arc<DeviceShared>) -> Self {
        Self { raw, device }
    }

    pub fn begin(&self) -> Result<()> {
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe {
            self.device
                .raw
                .begin_command_buffer(self.raw, &begin_info)?
        };

        Ok(())
    }

    pub fn end(&self) -> Result<()> {
        unsafe {
            self.device.raw.end_command_buffer(self.raw)?;
        }

        Ok(())
    }

    pub fn begin_rendering(
        &self,
        color_attachments: &[vk::RenderingAttachmentInfo],
        depth_attachment: Option<&vk::RenderingAttachmentInfo>,
        render_area: vk::Rect2D,
    ) {
        let empty_depth_attachment = vk::RenderingAttachmentInfo::default();

        let rendering_info = vk::RenderingInfo::default()
            .flags(vk::RenderingFlags::empty())
            .color_attachments(color_attachments)
            .depth_attachment(depth_attachment.unwrap_or_else(|| &empty_depth_attachment))
            .render_area(render_area)
            .layer_count(1);
        unsafe {
            self.device
                .raw
                .cmd_begin_rendering(self.raw, &rendering_info);
        }
    }

    pub fn end_rendering(&self) {
        unsafe {
            self.device.raw.cmd_end_rendering(self.raw);
        }
    }

    pub fn pipeline_image_barrier(&self, image_memory_barriers: &[vk::ImageMemoryBarrier2]) {
        let dependency_info =
            vk::DependencyInfo::default().image_memory_barriers(image_memory_barriers);
        unsafe {
            self.device
                .raw
                .cmd_pipeline_barrier2(self.raw, &dependency_info);
        }
    }

    pub fn bind_pipeline_graphics(&self, pipeline: &Pipeline) {
        unsafe {
            self.device.raw.cmd_bind_pipeline(
                self.raw,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.raw,
            );
        }
    }

    pub fn bind_descriptor_set_graphics(
        &self,
        descriptor_set: &DescriptorSet,
        pipeline: &Pipeline,
    ) {
        unsafe {
            self.device.raw.cmd_bind_descriptor_sets(
                self.raw,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.raw_layout,
                0,
                std::slice::from_ref(&descriptor_set.raw),
                &[],
            )
        }
    }

    pub fn bind_descriptor_sets_graphics(
        &self,
        descriptor_sets: &[&DescriptorSet],
        pipeline: &Pipeline,
    ) {
        unsafe {
            self.device.raw.cmd_bind_descriptor_sets(
                self.raw,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.raw_layout,
                0,
                &descriptor_sets
                    .iter()
                    .map(|set| set.raw)
                    .collect::<Vec<_>>(),
                &[],
            )
        }
    }

    pub fn bind_vertex_buffers(&self, first_binding: u32, buffers: &[&Buffer], offsets: &[u64]) {
        let raw_buffers = buffers.iter().map(|buffer| buffer.raw).collect::<Vec<_>>();
        unsafe {
            self.device.raw.cmd_bind_vertex_buffers2(
                self.raw,
                first_binding,
                &raw_buffers,
                offsets,
                None,
                None,
            )
        }
    }

    pub fn bind_index_buffer(&self, buffer: &Buffer, offset: u64) {
        unsafe {
            self.device.raw.cmd_bind_index_buffer(
                self.raw,
                buffer.raw,
                offset,
                vk::IndexType::UINT16,
            );
        }
    }

    pub fn draw(
        &self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        unsafe {
            self.device.raw.cmd_draw(
                self.raw,
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
            );
        }
    }

    pub fn draw_indexed(
        &self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) {
        unsafe {
            self.device.raw.cmd_draw_indexed(
                self.raw,
                index_count,
                instance_count,
                first_index,
                vertex_offset,
                first_instance,
            );
        }
    }

    pub fn draw_indirect(&self, buffer: &Buffer, offset: u64, draw_count: u32, stride: u32) {
        unsafe {
            self.device
                .raw
                .cmd_draw_indirect(self.raw, buffer.raw, offset, draw_count, stride)
        }
    }

    pub fn draw_indirect_count(
        &self,
        buffer: &Buffer,
        buffer_offset: u64,
        count_buffer: &Buffer,
        count_buffer_offset: u64,
        max_draw_count: u32,
        stride: u32,
    ) {
        unsafe {
            self.device.raw.cmd_draw_indirect_count(
                self.raw,
                buffer.raw,
                buffer_offset,
                count_buffer.raw,
                count_buffer_offset,
                max_draw_count,
                stride,
            )
        }
    }

    pub fn draw_indexed_indirect(
        &self,
        buffer: &Buffer,
        offset: u64,
        draw_count: u32,
        stride: u32,
    ) {
        unsafe {
            self.device
                .raw
                .cmd_draw_indexed_indirect(self.raw, buffer.raw, offset, draw_count, stride)
        }
    }

    pub fn draw_indexed_indirect_count(
        &self,
        buffer: &Buffer,
        buffer_offset: u64,
        count_buffer: &Buffer,
        count_buffer_offset: u64,
        max_draw_count: u32,
        stride: u32,
    ) {
        unsafe {
            self.device.raw.cmd_draw_indexed_indirect_count(
                self.raw,
                buffer.raw,
                buffer_offset,
                count_buffer.raw,
                count_buffer_offset,
                max_draw_count,
                stride,
            )
        }
    }

    pub fn draw_mesh_tasks(&self, task_count: u32, first_task: u32) {
        unsafe {
            self.device
                .mesh_shader_functions
                .cmd_draw_mesh_tasks(self.raw, task_count, first_task);
        }
    }

    pub fn draw_mesh_tasks_indirect(
        &self,
        buffer: &Buffer,
        offset: u64,
        draw_count: u32,
        stride: u32,
    ) {
        unsafe {
            self.device
                .mesh_shader_functions
                .cmd_draw_mesh_tasks_indirect(self.raw, buffer.raw, offset, draw_count, stride)
        }
    }

    pub fn draw_mesh_tasks_indirect_count(
        &self,
        buffer: &Buffer,
        buffer_offset: u64,
        count_buffer: &Buffer,
        count_buffer_offset: u64,
        max_draw_count: u32,
        stride: u32,
    ) {
        unsafe {
            self.device
                .mesh_shader_functions
                .cmd_draw_mesh_tasks_indirect_count(
                    self.raw,
                    buffer.raw,
                    buffer_offset,
                    count_buffer.raw,
                    count_buffer_offset,
                    max_draw_count,
                    stride,
                )
        }
    }

    pub fn copy_buffer_to_image(&self, buffer: &Buffer, image: &Image, buffer_offset: u64) {
        let region = vk::BufferImageCopy2::default()
            .buffer_offset(buffer_offset)
            .buffer_row_length(0)
            .buffer_image_height(0)
            // XXX: Handle subresource copy properly
            .image_subresource(
                vk::ImageSubresourceLayers::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .mip_level(0)
                    .base_array_layer(0)
                    .layer_count(1),
            )
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(image.extent);

        let info = vk::CopyBufferToImageInfo2::default()
            .src_buffer(buffer.raw)
            .dst_image(image.raw)
            .dst_image_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .regions(std::slice::from_ref(&region));

        unsafe {
            self.device.raw.cmd_copy_buffer_to_image2(self.raw, &info);
        }
    }
}

impl Device {
    pub fn get_current_command_buffer(&self) -> Result<CommandBuffer> {
        self.command_buffer_manager
            .lock()
            .get_command_buffer_at_pool(self.frame_counters.read().current as _)
    }

    /// Starts dynamic rendering on the current swapchain image. Note that `Device` holds all surface/swapchain resources internally,
    /// hence it makes the most sense to put this command directly on the device.
    pub fn command_begin_rendering_swapchain(
        &self,
        command_buffer: &CommandBuffer,
        clear_color: [f32; 4],
        image_depth: Option<&Image>,
    ) {
        let swapchain = self.swapchain.lock();
        let swapchain_color_attachment = vk::RenderingAttachmentInfo::default()
            .image_view(swapchain.current_image_view_raw())
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .resolve_mode(vk::ResolveModeFlags::NONE)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: clear_color,
                },
            });
        let swapchain_render_area = vk::Rect2D {
            extent: swapchain.extent,
            offset: vk::Offset2D { x: 0, y: 0 },
        };

        let swapchain_depth_attachment = image_depth.map(|image| {
            vk::RenderingAttachmentInfo::default()
                .image_view(image.raw_view)
                .image_layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
                .resolve_mode(vk::ResolveModeFlags::NONE)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                })
        });

        command_buffer.begin_rendering(
            &[swapchain_color_attachment],
            swapchain_depth_attachment.as_ref(),
            swapchain_render_area,
        );
    }

    /// Swapchain image layout needs manual image transition. These are aux helper functions to do those.
    pub fn command_transition_swapchain_image_layout_to_color_attachment(
        &self,
        command_buffer: &CommandBuffer,
    ) {
        let swapchain = self.swapchain.lock();

        let image_memory_barrier = vk::ImageMemoryBarrier2::default()
            .src_access_mask(vk::AccessFlags2::NONE)
            .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
            .src_stage_mask(vk::PipelineStageFlags2::empty())
            .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .image(swapchain.current_image_raw())
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            );

        command_buffer.pipeline_image_barrier(&[image_memory_barrier]);
    }

    pub fn command_transition_swapchain_image_layout_to_present(
        &self,
        command_buffer: &CommandBuffer,
    ) {
        let swapchain = self.swapchain.lock();

        let image_memory_barrier = vk::ImageMemoryBarrier2::default()
            .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
            .dst_access_mask(vk::AccessFlags2::NONE)
            .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(vk::PipelineStageFlags2::empty())
            .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .image(swapchain.current_image_raw())
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            );

        command_buffer.pipeline_image_barrier(&[image_memory_barrier]);
    }

    /// Slow copy data to image and set it up for shader read resource.
    pub fn upload_data_to_image_slow<T: Copy>(
        self: &Arc<Self>,
        image: &Image,
        data: &[T],
    ) -> Result<()> {
        let staging_buffer = self.create_buffer(BufferDescriptor {
            size: (std::mem::size_of::<T>() * data.len()) as _,
            usage_flags: vk::BufferUsageFlags::TRANSFER_SRC,
            memory_location: MemoryLocation::CpuToGpu,
        })?;

        staging_buffer.write_data(data)?;

        // XXX: Have proper command pool here.
        let command_buffer_raw = self.command_buffer_manager.lock().command_pools[0]
            .allocate_command_buffers(vk::CommandBufferLevel::PRIMARY, 1)?[0];
        let command_buffer =
            CommandBuffer::new_from_vulkan_handle(command_buffer_raw, self.shared.clone());

        command_buffer.begin()?;

        // XXX: Properly determine this.
        let subresource_range = vk::ImageSubresourceRange::default()
            .base_mip_level(0)
            .level_count(1)
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_array_layer(0)
            .layer_count(1);

        let image_barrier = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::NONE)
            .src_access_mask(vk::AccessFlags2::NONE)
            .dst_stage_mask(vk::PipelineStageFlags2::COPY)
            .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .subresource_range(subresource_range)
            .image(image.raw);
        command_buffer.pipeline_image_barrier(&[image_barrier]);

        command_buffer.copy_buffer_to_image(&staging_buffer, image, 0);

        let image_barrier = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::COPY)
            .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags2::SHADER_READ)
            .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .subresource_range(subresource_range)
            .image(image.raw);
        command_buffer.pipeline_image_barrier(&[image_barrier]);

        command_buffer.end()?;

        self.queue_graphics_present
            .submit_command_buffers(&[command_buffer.raw], &[], &[])?;

        self.queue_wait_idle(self.queue_graphics_present.raw)?;

        Ok(())
    }
}
