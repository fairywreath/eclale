use std::sync::{Arc, Mutex};

use anyhow::Result;
use ash::vk;
use egui_ash_renderer::{DynamicRendering, Options, Renderer};
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};

use crate::vulkan::device::Device;

pub use egui_ash_renderer;

pub(crate) struct GuiRendererDesc {
    pub(crate) depth_attachment_format: Option<vk::Format>,
}

/// Immediate mode GUI renderer.
pub struct GuiRenderer {
    pub renderer: Renderer,
    pub device: Arc<Device>,
}

impl GuiRenderer {
    pub(crate) fn new(device: Arc<Device>, desc: GuiRendererDesc) -> Result<Self> {
        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: device.shared.instance.raw.clone(),
            device: device.shared.raw.clone(),
            physical_device: device.shared.physical_device.raw,
            debug_settings: Default::default(),
            buffer_device_address: false,
            allocation_sizes: Default::default(),
        })?;

        let renderer = Renderer::with_gpu_allocator(
            Arc::new(Mutex::new(allocator)),
            device.shared.raw.clone(),
            DynamicRendering {
                color_attachment_format: device.swapchain.lock().surface_format.format,
                depth_attachment_format: desc.depth_attachment_format,
            },
            Options {
                srgb_framebuffer: true,
                ..Default::default()
            },
        )?;

        Ok(Self { device, renderer })
    }
}

