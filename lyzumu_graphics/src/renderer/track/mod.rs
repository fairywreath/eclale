use std::{mem::size_of, sync::Arc};

use anyhow::Result;
use hit_object::HitObjectRenderer;
use nalgebra::{Matrix4, Vector2};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use crate::{
    scene::TrackScene,
    vulkan::{
        device::Device,
        gpu_allocator::MemoryLocation,
        resource::{Buffer, BufferDescriptor, Image, ImageDescriptor},
        vk,
    },
};

mod hit_object;

pub(crate) struct HitObjectSharedContext {
    pub(crate) image_depth: Image,
    pub(crate) uniform_buffer_global: Arc<Buffer>,
}

#[derive(Clone, Copy, Default)]
pub struct TrackUniformBufferData {
    pub view_projection: Matrix4<f32>,
    pub runner_transform: Matrix4<f32>,
}

pub struct TrackRenderer {
    device: Arc<Device>,

    uniform_buffer_global_data: TrackUniformBufferData,
    uniform_buffer_global: Arc<Buffer>,

    hit_object_shared: Arc<HitObjectSharedContext>,
    hit_object_renderer: HitObjectRenderer,
}

impl TrackRenderer {
    pub fn new(window_handle: RawWindowHandle, display_handle: RawDisplayHandle) -> Result<Self> {
        let device = Arc::new(Device::new(window_handle, display_handle)?);

        let uniform_buffer_global = device.create_buffer(BufferDescriptor {
            size: size_of::<TrackUniformBufferData>() as _,
            usage_flags: vk::BufferUsageFlags::UNIFORM_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        })?;
        let uniform_buffer_global = Arc::new(uniform_buffer_global);

        let image_depth = Self::create_image_depth(&device)?;
        let hit_object_shared = Arc::new(HitObjectSharedContext {
            image_depth,
            uniform_buffer_global: uniform_buffer_global.clone(),
        });
        let hit_object_renderer =
            HitObjectRenderer::new(device.clone(), hit_object_shared.clone())?;

        log::debug!("TrackRenderer successfully created");

        Ok(Self {
            device,

            uniform_buffer_global_data: Default::default(),
            uniform_buffer_global,

            hit_object_shared,
            hit_object_renderer,
        })
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

    pub fn update_scene_constants(&mut self, scene_constants: TrackUniformBufferData) {
        self.uniform_buffer_global_data = scene_constants;
    }

    pub fn render(&mut self) -> Result<()> {
        self.device.frame_begin()?;

        // XXX FIXME: potentially dangerous as uniform buffer may still be read as we are writing
        // to it.
        self.uniform_buffer_global
            .write_data(std::slice::from_ref(&self.uniform_buffer_global_data))?;

        let commands = self.device.get_current_command_buffer()?;

        commands.begin()?;
        self.device
            .command_transition_swapchain_image_layout_to_color_attachment(&commands);
        self.device.command_begin_rendering_swapchain(
            &commands,
            [1.0, 1.0, 1.0, 1.0],
            Some(&self.hit_object_shared.image_depth),
        );

        self.hit_object_renderer
            .write_render_commands(&commands, self.device.current_frame())?;

        commands.end_rendering();
        self.device
            .command_transition_swapchain_image_layout_to_present(&commands);
        commands.end()?;

        self.device.queue_submit_commands_graphics(commands)?;
        self.device.swapchain_present()?;

        Ok(())
    }

    pub fn load_scene(&mut self, scene: TrackScene) -> Result<()> {
        self.hit_object_renderer.load_scene(scene)
    }

    pub fn swapchain_extent(&self) -> Vector2<u32> {
        Vector2::new(
            self.device.swapchain_extent().width,
            self.device.swapchain_extent().height,
        )
    }
}
