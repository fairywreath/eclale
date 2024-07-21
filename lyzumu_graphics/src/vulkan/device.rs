use std::sync::Arc;

use anyhow::{Context, Result};
use ash::vk::{self};
use parking_lot::{Mutex, RwLock};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use super::resource::{PendingDestructionImage, PendingDestructionSampler};

use super::{
    command::{CommandBuffer, CommandBufferManager},
    resource::{DescriptorPool, PendingDestructionBuffer, PendingDestructionPipeline},
    DeviceShared, Instance, Queue, QueueSubmitSemaphoreDescriptor, Semaphore, SemaphoreType,
    Surface, Swapchain, QUEUE_FAMILY_INDEX_GRAPHICS,
};

pub const MAX_FRAMES: usize = 2;
pub const GLOBAL_DESCRIPTOR_POOL_DESCRIPTOR_COUNT: u32 = 128;
pub const GLOBAL_DESCRIPTOR_POOL_BINDLESS_TEXTURES_DESCRIPTOR_COUNT: u32 = 2048;

pub(crate) struct FrameCounters {
    pub(crate) current: u64,
    pub(crate) previous: u64,
    pub(crate) absolute: u64,
}

pub(crate) struct ResourceHub {
    pub(crate) pending_destruction_buffers: Vec<PendingDestructionBuffer>,
    pub(crate) pending_destruction_images: Vec<PendingDestructionImage>,
    pub(crate) pending_destruction_sampler: Vec<PendingDestructionSampler>,
    pub(crate) pending_destruction_pipelines: Vec<PendingDestructionPipeline>,
}

/// Structure that describes the functionality of a logical device and contains all the necessary resources
/// for rendering, including window/surface resources and swapchain recreation.
///
/// Handles frame synchronization logic and frame/thread resource management, such as command pools.
/// Preallocates all required command buffers.
///
/// Should be used/passed around as an immutable reference and members are internally mutable as required.
pub struct Device {
    pub(crate) resource_hub: Mutex<ResourceHub>,
    pub(crate) command_buffer_manager: Mutex<CommandBufferManager>,

    /// All descriptor sets are allocated from this pool. Can improve this to a growable allocator if required.
    pub(crate) global_descriptor_pool: DescriptorPool,
    pub(crate) global_descriptor_pool_bindless_textures: DescriptorPool,

    /// Frame synchronization device resources.
    ///
    /// Signal when queue submission is done, wait on this semaphore when presenting.
    semaphores_render_complete: [Semaphore; MAX_FRAMES],
    /// Signal semaphore when acquiring swapchain image, wait when submitting graphics command buffer work.
    semaphores_swapchain_image_acquired: [Semaphore; MAX_FRAMES],
    /// Timeline semaphore for general purpose rendering work. Only one semaphore required for (potentially) multiple frames in flight.
    semaphore_graphics_frame: Semaphore,

    pub(crate) frame_counters: RwLock<FrameCounters>,

    /// Same HW queue family for both graphics and present work.
    pub(crate) queue_graphics_present: Queue,

    pub(crate) swapchain: Mutex<Swapchain>,
    pub(crate) shared: Arc<DeviceShared>,
}

impl Device {
    pub fn new(window_handle: RawWindowHandle, display_handle: RawDisplayHandle) -> Result<Self> {
        let instance = Instance::new(display_handle)?;
        let surface = Surface::new(&instance, window_handle, display_handle)?;
        let shared = Arc::new(DeviceShared::new(instance, surface)?);
        let swapchain = Mutex::new(Swapchain::new(
            shared.clone(),
            vk::PresentModeKHR::IMMEDIATE,
        )?);

        // Always get index at queue 0 since only 1 queue is used per family.
        let queue_graphics_present_family_index =
            shared.queue_families[QUEUE_FAMILY_INDEX_GRAPHICS].index;
        let queue_graphics_present = unsafe {
            shared
                .raw
                .get_device_queue(queue_graphics_present_family_index, 0)
        };
        let queue_graphics_present = Queue::new_from_vulkan_handle(
            shared.raw.clone(),
            queue_graphics_present,
            queue_graphics_present_family_index,
        );

        let semaphores_render_complete = [
            Semaphore::new(shared.clone(), SemaphoreType::Binary)?,
            Semaphore::new(shared.clone(), SemaphoreType::Binary)?,
        ];
        let semaphores_swapchain_image_acquired = [
            Semaphore::new(shared.clone(), SemaphoreType::Binary)?,
            Semaphore::new(shared.clone(), SemaphoreType::Binary)?,
        ];

        let semaphore_graphics_frame = Semaphore::new(shared.clone(), SemaphoreType::Timeline)?;

        let command_buffer_manager = Mutex::new(CommandBufferManager::new(
            shared.clone(),
            MAX_FRAMES as _,
            1,
        )?);

        let resource_hub = Mutex::new(ResourceHub {
            pending_destruction_buffers: Vec::new(),
            pending_destruction_images: Vec::new(),
            pending_destruction_sampler: Vec::new(),
            pending_destruction_pipelines: Vec::new(),
        });

        let global_descriptor_pool_sizes = vec![
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(GLOBAL_DESCRIPTOR_POOL_DESCRIPTOR_COUNT),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(GLOBAL_DESCRIPTOR_POOL_DESCRIPTOR_COUNT),
        ];
        let global_descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(2048)
            .pool_sizes(&global_descriptor_pool_sizes);
        let global_descriptor_pool =
            DescriptorPool::new(shared.clone(), global_descriptor_pool_create_info)?;

        let global_descriptor_pool_bindless_textures =
            Self::create_descriptor_pool_bindless_textures(shared.clone())?;

        Ok(Self {
            shared,
            swapchain,
            queue_graphics_present,
            semaphore_graphics_frame,
            semaphores_swapchain_image_acquired,
            semaphores_render_complete,
            frame_counters: RwLock::new(FrameCounters {
                current: 0,
                previous: 0,
                absolute: 0,
            }),
            command_buffer_manager,
            resource_hub,
            global_descriptor_pool,
            global_descriptor_pool_bindless_textures,
        })
    }

    fn frame_counters_advance(&self) {
        let mut counters = self.frame_counters.write();
        counters.previous = counters.current;
        counters.current = (counters.current + 1) % (MAX_FRAMES as u64);
        counters.absolute += 1;
    }

    /// Returns the timeline semaphore value needed to be waited on before beggining a frame.
    /// A "frame" shares GPU resources.
    fn frame_semaphore_graphics_wait_value(&self) -> u64 {
        self.frame_counters.read().absolute - (MAX_FRAMES as u64 - 1)
    }

    /// Additionally handles swapchain recreation when image acquisition fails.
    pub fn frame_begin(&self) -> Result<()> {
        // Ugly if statement where we only wait if we exceed the first set of MAX_FRAMES
        // as the first set does not have any graphics work beforehand.
        //
        // Need to wait for this timeline semaphore before resetting the command pool.
        // We technically can call the command pool reset somehwere else, for example when grabbing
        // the first command buffer for the frame.
        if self.frame_counters.read().absolute >= MAX_FRAMES as u64 {
            let graphics_wait_value = self.frame_semaphore_graphics_wait_value();

            let wait_values = [graphics_wait_value];
            let semaphores = [self.semaphore_graphics_frame.raw];

            let wait_info = vk::SemaphoreWaitInfo::default()
                .semaphores(&semaphores)
                .values(&wait_values);

            unsafe { self.shared.raw.wait_semaphores(&wait_info, u64::MAX)? };
        }

        let current_frame = self.frame_counters.read().current as usize;
        self.command_buffer_manager
            .lock()
            .reset_command_pools(&[current_frame as _])?;

        let mut swapchain = self.swapchain.lock();

        match swapchain
            .acquire_next_image(self.semaphores_swapchain_image_acquired[current_frame].raw)
        {
            Ok((_, true)) | Err(_) => {
                // XXX: Currently assume all errors are recreation requirement errors. Handle other errors as well.
                // For improvements, recreate when the actual window systems detects a window resized instead of
                // guessing the resize through acquire_next_image error internally here.
                log::debug!("Failed swapchain acquire next image!");
                swapchain.recreate()?;
                swapchain
                    .acquire_next_image(self.semaphores_swapchain_image_acquired[current_frame].raw)
                    .with_context(|| "Failed swapchain acquire next image after recreation!")?;
            }
            _ => {}
        };

        Ok(())
    }

    pub fn swapchain_present(&self) -> Result<()> {
        let swapchain = self.swapchain.lock();

        if let Err(_) = swapchain.queue_present(
            self.queue_graphics_present.raw,
            &[self.semaphores_render_complete[self.frame_counters.read().current as usize].raw],
        ) {
            // XXX: Currently assume all errors are swapchain out of date/required recreation errors.
            // Wait idle here and expect the swapchain recreation to fix this error in the next frame.
            // Handle all vk errors properly in the future.
            unsafe {
                self.shared.raw.device_wait_idle()?;
            }
        }

        self.frame_counters_advance();

        self.cleanup_resources()?;

        Ok(())
    }

    pub fn swapchain_extent(&self) -> vk::Extent2D {
        self.swapchain.lock().extent
    }

    pub fn swapchain_color_format(&self) -> vk::Format {
        self.swapchain.lock().surface_format.format
    }

    pub fn current_frame(&self) -> u64 {
        self.frame_counters.read().current
    }

    /// Submit commands to the dedicated graphics queue for per-frame rendering work.
    pub fn queue_submit_commands_graphics(&self, command_buffer: CommandBuffer) -> Result<()> {
        let current_frame = self.frame_counters.read().current as usize;
        let wait_semaphores = vec![QueueSubmitSemaphoreDescriptor {
            semaphore: &self.semaphores_swapchain_image_acquired[current_frame],
            stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            value: None,
        }];

        let signal_semaphores = [
            QueueSubmitSemaphoreDescriptor {
                semaphore: &self.semaphores_render_complete[current_frame], // XXX: Similar read as above but on a different line.... need to make sure they are the same
                stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                value: None,
            },
            // Signal per-frame/thread command buffer is ready to be used.
            QueueSubmitSemaphoreDescriptor {
                semaphore: &self.semaphore_graphics_frame,
                stage_mask: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                value: Some(self.frame_counters.read().absolute + 1), // XXX: Similar read as above but on a different line.... need to make sure they are the same
            },
        ];

        self.queue_graphics_present.submit_command_buffers(
            &[command_buffer.raw],
            &wait_semaphores,
            &signal_semaphores,
        )?;

        Ok(())
    }

    fn cleanup_resources(&self) -> Result<()> {
        let mut resource_hub = self.resource_hub.lock();
        for buffer in resource_hub.pending_destruction_buffers.drain(..) {
            self.destroy_buffer(buffer)?;
        }
        for image in resource_hub.pending_destruction_images.drain(..) {
            self.destroy_image(image)?;
        }
        for sampler in resource_hub.pending_destruction_sampler.drain(..) {
            self.destroy_sampler(sampler)?;
        }
        for pipeline in resource_hub.pending_destruction_pipelines.drain(..) {
            self.destroy_pipeline(pipeline)?;
        }

        Ok(())
    }

    fn create_descriptor_pool_bindless_textures(
        shared: Arc<DeviceShared>,
    ) -> Result<DescriptorPool> {
        let descriptor_pool_sizes = vec![
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(GLOBAL_DESCRIPTOR_POOL_BINDLESS_TEXTURES_DESCRIPTOR_COUNT),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(GLOBAL_DESCRIPTOR_POOL_BINDLESS_TEXTURES_DESCRIPTOR_COUNT),
        ];
        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(512)
            .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
            .pool_sizes(&descriptor_pool_sizes);
        Ok(DescriptorPool::new(
            shared.clone(),
            descriptor_pool_create_info,
        )?)
    }

    pub(crate) fn queue_wait_idle(&self, queue: vk::Queue) -> Result<()> {
        unsafe {
            self.shared.raw.queue_wait_idle(queue)?;
        };

        Ok(())
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.shared.raw.device_wait_idle().unwrap();
        }

        self.cleanup_resources().unwrap();
    }
}
