use std::{mem::size_of, sync::Arc};

use anyhow::Result;
use nalgebra::Vector3;

use crate::vulkan::{
    command::CommandBuffer,
    device::Device,
    gpu_allocator::MemoryLocation,
    resource::{
        Buffer, BufferDescriptor, DescriptorBindingBufferWrite, DescriptorBindingWrites,
        DescriptorSet, DescriptorSetDescriptor, DescriptorSetLayout, DescriptorSetPoolType,
        Pipeline,
    },
    vk,
};

use super::{render_description::MOSVDrawData, SharedGpuResources};

struct GpuResources {
    vertex_buffer_positions: Buffer,
    vertex_buffer_objects_indices: Buffer,
    index_buffer: Buffer,
    storage_buffer_objects: Buffer,
}

fn create_gpu_resources(device: &Arc<Device>, draw_data: &MOSVDrawData) -> Result<GpuResources> {
    // XXX TODO: Make all buffers GPU memory only.
    let storage_buffer_objects = device.create_buffer(BufferDescriptor {
        size: (draw_data.objects_data.len() * size_of::<u8>()) as u64,
        usage_flags: vk::BufferUsageFlags::STORAGE_BUFFER,
        memory_location: MemoryLocation::CpuToGpu,
    })?;
    storage_buffer_objects.write_data(&draw_data.objects_data)?;

    let vertex_buffer_positions = device.create_buffer(BufferDescriptor {
        size: (draw_data.vertices.len() * size_of::<Vector3<f32>>()) as u64,
        usage_flags: vk::BufferUsageFlags::VERTEX_BUFFER,
        memory_location: MemoryLocation::CpuToGpu,
    })?;
    vertex_buffer_positions.write_data(&draw_data.vertices)?;

    log::info!(
        "MOSV vertices {} indices {}",
        draw_data.vertices.len(),
        draw_data.indices.len()
    );

    let vertex_buffer_objects_indices = device.create_buffer(BufferDescriptor {
        size: (draw_data.vertices.len() * size_of::<Vector3<u32>>()) as u64,
        usage_flags: vk::BufferUsageFlags::VERTEX_BUFFER,
        memory_location: MemoryLocation::CpuToGpu,
    })?;
    vertex_buffer_objects_indices.write_data(&draw_data.objects_indices)?;

    let index_buffer = device.create_buffer(BufferDescriptor {
        size: (draw_data.indices.len() * size_of::<u16>()) as u64,
        usage_flags: vk::BufferUsageFlags::INDEX_BUFFER,
        memory_location: MemoryLocation::CpuToGpu,
    })?;
    index_buffer.write_data(&draw_data.indices)?;

    Ok(GpuResources {
        vertex_buffer_positions,
        vertex_buffer_objects_indices,
        index_buffer,
        storage_buffer_objects,
    })
}

pub(crate) struct MOSVRenderer {
    pub(crate) draw_data: MOSVDrawData,
    gpu_resources: GpuResources,
    descriptor_set: DescriptorSet,

    device: Arc<Device>,
}

impl MOSVRenderer {
    pub(crate) fn new(
        device: Arc<Device>,
        draw_data: MOSVDrawData,
        descriptor_set_layout: Arc<DescriptorSetLayout>,
        shared_resources: &SharedGpuResources,
    ) -> Result<Self> {
        let descriptor_set = device.create_descriptor_set(DescriptorSetDescriptor {
            layout: descriptor_set_layout.clone(),
            pool_type: DescriptorSetPoolType::GlobalGenericResource,
        })?;
        let gpu_resources = create_gpu_resources(&device, &draw_data)?;

        Self::update_descriptor_set(&device, &descriptor_set, &gpu_resources, shared_resources)?;

        Ok(Self {
            draw_data,
            gpu_resources,
            descriptor_set,
            device,
        })
    }

    pub(crate) fn record_draw_commands(
        &self,
        command_buffer: &CommandBuffer,
        graphics_pipeline: &Pipeline,
        _current_frame: u64,
    ) -> Result<()> {
        let draw_indexed_command = self.get_default_draw_indexed_command();

        command_buffer.bind_descriptor_set_graphics(&self.descriptor_set, &graphics_pipeline);

        command_buffer.bind_vertex_buffers(0, &[&self.gpu_resources.vertex_buffer_positions], &[0]);
        command_buffer.bind_vertex_buffers(
            1,
            &[&self.gpu_resources.vertex_buffer_objects_indices],
            &[0],
        );

        command_buffer.bind_index_buffer(&self.gpu_resources.index_buffer, 0);
        command_buffer.draw_indexed(
            draw_indexed_command.index_count,
            draw_indexed_command.instance_count,
            draw_indexed_command.first_index,
            draw_indexed_command.vertex_offset,
            draw_indexed_command.first_instance,
        );

        Ok(())
    }

    fn get_default_draw_indexed_command(&self) -> vk::DrawIndexedIndirectCommand {
        vk::DrawIndexedIndirectCommand::default()
            .index_count(self.draw_data.indices.len() as _)
            .instance_count(1)
    }

    fn update_descriptor_set(
        device: &Device,
        descriptor_set: &DescriptorSet,
        instanced_resources: &GpuResources,
        shared_resources: &SharedGpuResources,
    ) -> Result<()> {
        let descriptor_binding_writes = DescriptorBindingWrites {
            buffers: vec![
                DescriptorBindingBufferWrite {
                    buffer: &shared_resources.uniform_buffer_global,
                    binding_index: 0,
                },
                DescriptorBindingBufferWrite {
                    buffer: &instanced_resources.storage_buffer_objects,
                    binding_index: 1,
                },
            ],
        };
        device.update_descriptor_set(descriptor_set, &descriptor_binding_writes)?;

        Ok(())
    }

    pub(crate) fn update_shared_gpu_resources(
        &self,
        shared_resources: &SharedGpuResources,
    ) -> Result<()> {
        Self::update_descriptor_set(
            &self.device,
            &self.descriptor_set,
            &self.gpu_resources,
            shared_resources,
        )
    }
}
