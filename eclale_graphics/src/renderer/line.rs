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

use super::{render_description::InstancedDrawData, SharedGpuResources};

pub(crate) struct LineRenderer {}
