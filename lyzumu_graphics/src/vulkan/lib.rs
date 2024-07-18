use std::{
    ffi::{c_void, CStr, CString},
    mem::ManuallyDrop,
    sync::Arc,
};

use anyhow::{Context, Result};
use ash::{ext::debug_utils, khr, nv::mesh_shader};
use gpu_allocator::{
    vulkan::{Allocator, AllocatorCreateDesc},
    AllocationSizes, AllocatorDebugSettings,
};
use parking_lot::Mutex;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

pub mod command;
pub mod device;
pub mod resource;
pub mod shader;
pub mod types;

/// External dependencies exposed outside of currrent crate.
pub use ash::{self, vk};
pub use gpu_allocator;
pub use raw_window_handle;

const QUEUE_FAMILY_INDEX_GRAPHICS: usize = 0;

struct Instance {
    entry: ash::Entry,
    raw: ash::Instance,
    debug_utils: debug_utils::Instance,
    debug_utils_messenger: vk::DebugUtilsMessengerEXT,
}

impl Instance {
    fn new(display_handle: RawDisplayHandle) -> Result<Self> {
        let entry = unsafe { ash::Entry::load()? };

        // Create Vulkan instance
        let app_name = CString::new("Rikka").unwrap();
        let app_info = vk::ApplicationInfo::default()
            .application_name(app_name.as_c_str())
            .api_version(vk::API_VERSION_1_3);

        let mut extension_names =
            ash_window::enumerate_required_extensions(display_handle)?.to_vec();
        extension_names.push(debug_utils::NAME.as_ptr());

        let layer_strings = vec![CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
        let layer_names: Vec<*const i8> =
            layer_strings.iter().map(|c_str| c_str.as_ptr()).collect();

        let instance_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extension_names)
            .enabled_layer_names(&layer_names);

        let raw = unsafe { entry.create_instance(&instance_info, None)? };

        // Create Vulkan debug utils messenger
        let debug_utils_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .flags(vk::DebugUtilsMessengerCreateFlagsEXT::empty())
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(vulkan_debug_utils_callback));

        let debug_utils = debug_utils::Instance::new(&entry, &raw);
        let debug_utils_messenger =
            unsafe { debug_utils.create_debug_utils_messenger(&debug_utils_info, None)? };

        Ok(Self {
            entry,
            raw,
            debug_utils,
            debug_utils_messenger,
        })
    }

    fn get_physical_devices(&self, surface: &Surface) -> Result<Vec<PhysicalDevice>> {
        let physical_devices = unsafe { self.raw.enumerate_physical_devices()? };
        physical_devices
            .into_iter()
            .map(|phys_device| {
                PhysicalDevice::new_from_vulkan_handle(&self.raw, &surface, phys_device)
            })
            .collect::<Result<_>>()
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        log::trace!("Instance dropped");
        unsafe {
            self.debug_utils
                .destroy_debug_utils_messenger(self.debug_utils_messenger, None);
            self.raw.destroy_instance(None);
        }
    }
}

unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    let severity = match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "[Verbose]",
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "[Warning]",
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "[Error]",
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "[Info]",
        _ => "[Unknown]",
    };
    let types = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[General]",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[Performance]",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[Validation]",
        _ => "[Unknown]",
    };
    let message = CStr::from_ptr((*p_callback_data).p_message);
    log::debug!("[VK Debug]{}{}{:?}", severity, types, message);

    vk::FALSE
}

#[derive(Debug, Clone)]
struct PhysicalDevice {
    raw: vk::PhysicalDevice,
    name: String,
    device_type: vk::PhysicalDeviceType,
    _limits: vk::PhysicalDeviceLimits,
    _properties: vk::PhysicalDeviceProperties,
    queue_families: Vec<QueueFamily>,
    _supported_extensions: Vec<String>,
    _supported_surface_formats: Vec<vk::SurfaceFormatKHR>,
    _supported_present_modes: Vec<vk::PresentModeKHR>,
}

impl PhysicalDevice {
    fn new_from_vulkan_handle(
        instance: &ash::Instance,
        surface: &Surface,
        raw: vk::PhysicalDevice,
    ) -> Result<Self> {
        let properties = unsafe { instance.get_physical_device_properties(raw) };
        let name = unsafe {
            CStr::from_ptr(properties.device_name.as_ptr())
                .to_str()
                .unwrap()
                .to_owned()
        };
        let device_type = properties.device_type;
        let limits = properties.limits;

        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(raw) };
        let queue_families = queue_family_properties
            .into_iter()
            .enumerate()
            .map(|(index, prop)| {
                let present_support = unsafe {
                    surface.raw_ash.get_physical_device_surface_support(
                        raw,
                        index as _,
                        surface.raw_vulkan,
                    )?
                };
                Ok(QueueFamily::new(index as _, prop, present_support))
            })
            .collect::<Result<_>>()?;

        let extension_properties = unsafe { instance.enumerate_device_extension_properties(raw)? };
        let supported_extensions = extension_properties
            .into_iter()
            .map(|prop| {
                let name = unsafe { CStr::from_ptr(prop.extension_name.as_ptr()) };
                name.to_str().unwrap().to_owned()
            })
            .collect();

        let supported_surface_formats = unsafe {
            surface
                .raw_ash
                .get_physical_device_surface_formats(raw, surface.raw_vulkan)?
        };

        let supported_present_modes = unsafe {
            surface
                .raw_ash
                .get_physical_device_surface_present_modes(raw, surface.raw_vulkan)?
        };

        Ok(Self {
            raw,
            name,
            device_type,
            _limits: limits,
            _properties: properties,
            queue_families,
            _supported_extensions: supported_extensions,
            _supported_surface_formats: supported_surface_formats,
            _supported_present_modes: supported_present_modes,
        })
    }

    fn _supports_extensions(&self, extensions: &[&str]) -> bool {
        let supported_extensions = self
            ._supported_extensions
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();

        extensions
            .iter()
            .all(|ext| supported_extensions.contains(ext))
    }
}

pub(crate) struct DeviceShared {
    pub(crate) allocator: ManuallyDrop<Mutex<Allocator>>,
    pub(crate) raw: ash::Device,
    pub(crate) mesh_shader_functions: mesh_shader::Device,
    queue_families: Vec<QueueFamily>,
    physical_device: PhysicalDevice,
    surface: Surface,
    instance: Instance,
}

impl DeviceShared {
    fn new(instance: Instance, surface: Surface) -> Result<Self> {
        let physical_devices = instance.get_physical_devices(&surface)?;
        let physical_device = select_discrete_gpu(&physical_devices)?;
        let queue_families = select_queue_families(&physical_device);

        log::info!("Physical device name: {}", physical_device.name);

        let raw = Self::new_ash_device(&instance, &physical_device, &queue_families)?;

        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: instance.raw.clone(),
            device: raw.clone(),
            physical_device: physical_device.raw,
            debug_settings: AllocatorDebugSettings {
                log_memory_information: true,
                log_leaks_on_shutdown: true,
                ..Default::default()
            },
            buffer_device_address: true,
            allocation_sizes: AllocationSizes::default(),
        })?;
        let allocator = Mutex::new(allocator);

        let mesh_shader_functions = mesh_shader::Device::new(&instance.raw, &raw);

        Ok(Self {
            allocator: ManuallyDrop::new(allocator),
            queue_families,
            raw,
            mesh_shader_functions,
            physical_device,
            surface,
            instance,
        })
    }

    fn new_ash_device(
        instance: &Instance,
        physical_device: &PhysicalDevice,
        queue_families: &[QueueFamily],
    ) -> Result<ash::Device> {
        let queue_priorities = [1.0f32];

        let queue_create_infos = {
            let mut indices = queue_families
                .iter()
                .map(|family| family.index)
                .collect::<Vec<_>>();

            indices.sort();
            indices.dedup();

            indices
                .iter()
                .map(|index| {
                    vk::DeviceQueueCreateInfo::default()
                        .queue_family_index(*index)
                        .queue_priorities(&queue_priorities)
                })
                .collect::<Vec<_>>()
        };

        let device_extension_strs = ["VK_KHR_swapchain", "VK_NV_mesh_shader"];
        let device_extension_strs = device_extension_strs
            .iter()
            .map(|str| CString::new(*str))
            .collect::<Result<Vec<_>, _>>()?;
        let device_extension_strs = device_extension_strs
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<_>>();

        let mut vulkan11_features = vk::PhysicalDeviceVulkan11Features::default()
            .shader_draw_parameters(true)
            .storage_buffer16_bit_access(true);
        let mut vulkan12_features = vk::PhysicalDeviceVulkan12Features::default()
            .descriptor_indexing(true)
            .runtime_descriptor_array(true)
            .descriptor_binding_partially_bound(true)
            .descriptor_binding_variable_descriptor_count(true)
            .descriptor_binding_sampled_image_update_after_bind(true)
            .descriptor_binding_storage_image_update_after_bind(true)
            .timeline_semaphore(true)
            .shader_sampled_image_array_non_uniform_indexing(true)
            .buffer_device_address(true)
            .storage_buffer8_bit_access(true);
        let mut vulkan13_features = vk::PhysicalDeviceVulkan13Features::default()
            .dynamic_rendering(true)
            .synchronization2(true);

        let mut mesh_shader_features = vk::PhysicalDeviceMeshShaderFeaturesNV::default()
            .mesh_shader(true)
            .task_shader(true);

        // PhysicalDeviceFeatures 2 reports ALL of GPU's device features capabilies. Pass this along pNext chain to enable all.
        let mut device_features2 = vk::PhysicalDeviceFeatures2::default();
        unsafe {
            instance
                .raw
                .get_physical_device_features2(physical_device.raw, &mut device_features2);
        }
        device_features2 = device_features2
            .push_next(&mut vulkan11_features)
            .push_next(&mut vulkan12_features)
            .push_next(&mut vulkan13_features)
            .push_next(&mut mesh_shader_features);

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_extension_strs)
            .push_next(&mut device_features2);

        // Create vulkan logical device.
        let device = unsafe {
            instance
                .raw
                .create_device(physical_device.raw, &device_create_info, None)?
        };

        Ok(device)
    }
}

impl Drop for DeviceShared {
    fn drop(&mut self) {
        unsafe {
            log::trace!("Logical device dropped");
            ManuallyDrop::drop(&mut self.allocator);
            self.raw.destroy_device(None);
        }
    }
}

/// Selects the first discrete GPU found from the list of physical devices.
fn select_discrete_gpu(devices: &[PhysicalDevice]) -> Result<PhysicalDevice> {
    let device = devices
        .iter()
        .find(|device| device.device_type == vk::PhysicalDeviceType::DISCRETE_GPU)
        .ok_or_else(|| anyhow::anyhow!("Discrete GPU not found!"))?;

    Ok(device.clone())
}
struct Surface {
    raw_ash: khr::surface::Instance,
    raw_vulkan: vk::SurfaceKHR,
}

impl Surface {
    fn new(
        instance: &Instance,
        window_handle: RawWindowHandle,
        display_handle: RawDisplayHandle,
    ) -> Result<Self> {
        let raw_ash = khr::surface::Instance::new(&instance.entry, &instance.raw);
        let raw_vulkan = unsafe {
            ash_window::create_surface(
                &instance.entry,
                &instance.raw,
                display_handle,
                window_handle,
                None,
            )?
        };

        Ok(Self {
            raw_ash,
            raw_vulkan,
        })
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.raw_ash.destroy_surface(self.raw_vulkan, None);
        }
    }
}

pub(crate) struct Swapchain {
    raw_ash: khr::swapchain::Device,
    raw_vulkan: vk::SwapchainKHR,
    images_raw: Vec<vk::Image>,
    pub(crate) image_views_raw: Vec<vk::ImageView>,
    pub(crate) image_index: u32,
    pub(crate) surface_format: vk::SurfaceFormatKHR,
    pub(crate) extent: vk::Extent2D,
    device: Arc<DeviceShared>,
}

impl Swapchain {
    fn new(device: Arc<DeviceShared>, requested_present_mode: vk::PresentModeKHR) -> Result<Self> {
        let surface_format = {
            let formats = unsafe {
                device.surface.raw_ash.get_physical_device_surface_formats(
                    device.physical_device.raw,
                    device.surface.raw_vulkan,
                )?
            };

            if formats.len() == 1 && formats[0].format == vk::Format::UNDEFINED {
                vk::SurfaceFormatKHR {
                    format: vk::Format::B8G8R8A8_UNORM,
                    color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
                }
            } else {
                *formats
                    .iter()
                    .find(|format| {
                        format.format == vk::Format::B8G8R8A8_UNORM
                            && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
                    })
                    .unwrap_or(&formats[0])
            }
        };

        let present_mode = {
            let present_modes = unsafe {
                device
                    .surface
                    .raw_ash
                    .get_physical_device_surface_present_modes(
                        device.physical_device.raw,
                        device.surface.raw_vulkan,
                    )?
            };

            if present_modes.contains(&requested_present_mode) {
                requested_present_mode
            } else {
                return Err(anyhow::anyhow!("Present mode not supported"));
            }
        };

        // Get surface capabilities.
        let capabilities = unsafe {
            device
                .surface
                .raw_ash
                .get_physical_device_surface_capabilities(
                    device.physical_device.raw,
                    device.surface.raw_vulkan,
                )?
        };

        let extent = {
            if capabilities.current_extent.width != std::u32::MAX {
                capabilities.current_extent
            } else {
                let max_extent = capabilities.max_image_extent;
                let width = max_extent.width;
                let height = max_extent.height;

                vk::Extent2D { width, height }
            }
        };

        let image_count = capabilities
            .max_image_count
            .min(capabilities.min_image_count + 1);

        log::debug!("Swapchain extent: {} X {}", extent.width, extent.height);

        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(device.surface.raw_vulkan)
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(
                vk::ImageUsageFlags::COLOR_ATTACHMENT
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::TRANSFER_SRC,
            )
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE) // Graphics and present queue are the same family
            .present_mode(present_mode);

        let raw_ash = khr::swapchain::Device::new(&device.instance.raw, &device.raw);
        let raw_vulkan = unsafe { raw_ash.create_swapchain(&create_info, None)? };

        let images_raw = unsafe { raw_ash.get_swapchain_images(raw_vulkan)? };
        let image_views_raw = images_raw
            .iter()
            .map(|image| {
                let image_view_info = vk::ImageViewCreateInfo::default()
                    .image(image.clone())
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(surface_format.format)
                    .components(
                        vk::ComponentMapping::default()
                            .r(vk::ComponentSwizzle::IDENTITY)
                            .g(vk::ComponentSwizzle::IDENTITY)
                            .b(vk::ComponentSwizzle::IDENTITY)
                            .a(vk::ComponentSwizzle::IDENTITY),
                    )
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1),
                    );

                Ok(unsafe { device.raw.create_image_view(&image_view_info, None)? })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            device,
            raw_ash,
            raw_vulkan,
            images_raw,
            image_views_raw,
            image_index: 0,
            surface_format,
            extent,
        })
    }

    fn acquire_next_image(&mut self, signal_semaphore: vk::Semaphore) -> Result<(u32, bool)> {
        let (image_index, is_suboptimal) = unsafe {
            self.raw_ash.acquire_next_image(
                self.raw_vulkan,
                u64::MAX - 1,
                signal_semaphore,
                vk::Fence::null(),
            )?
        };
        self.image_index = image_index;
        Ok((image_index, is_suboptimal))
    }

    /// Returns whether the swapchain is suboptimal for the susrface.
    fn queue_present(&self, queue: vk::Queue, wait_semaphores: &[vk::Semaphore]) -> Result<bool> {
        let swapchains = [self.raw_vulkan];
        let image_indices = [self.image_index];

        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        let result = unsafe {
            self.raw_ash
                .queue_present(queue, &present_info)
                .with_context(|| "Failed swapchain queue present!")?
        };

        Ok(result)
    }

    pub(crate) fn current_image_raw(&self) -> vk::Image {
        self.images_raw[self.image_index as usize]
    }

    pub(crate) fn current_image_view_raw(&self) -> vk::ImageView {
        self.image_views_raw[self.image_index as usize]
    }

    fn recreate(&mut self) -> Result<()> {
        self.destroy();
        log::debug!("Recreating swapchain...");
        let new_swapchain = Self::new(self.device.clone(), vk::PresentModeKHR::FIFO)?;
        *self = new_swapchain;
        log::debug!("Done recreating swapchain.");
        Ok(())
    }

    // Desstroys all internal swapchain resources. Should not be called publicly as the swapchain structure object itself
    // is left at a valid state. This function is useful when recreating swapchains.
    fn destroy(&mut self) {
        if !self.image_views_raw.is_empty() {
            unsafe {
                for image_view in self.image_views_raw.drain(..) {
                    self.device.raw.destroy_image_view(image_view, None);
                }

                self.raw_ash.destroy_swapchain(self.raw_vulkan, None);
            }
        }
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        self.destroy();
    }
}

/// Selects separate queue family indices for graphics, compute, and transfer functionality.
/// Returns 4 indices by queue usage in this order: graphics, present, compute and transfer.
fn select_queue_families(device: &PhysicalDevice) -> Vec<QueueFamily> {
    let mut graphics = None;
    let mut present = None;
    let mut compute = None;
    let mut transfer = None;

    // 1 graphics + present family, 1 compute family and 1 transfer only family
    for family in device
        .queue_families
        .iter()
        .filter(|family| family.properties.queue_count > 0)
    {
        if family.supports_graphics() && graphics.is_none() {
            graphics = Some(*family);
            // Assume graphics queue also supports present, and use it as main present queue as well
            assert!(family.supports_present);
            present = Some(*family);
        } else if family.supports_compute() && compute.is_none() {
            compute = Some(*family);
        } else if family.supports_transfer() && !family.supports_compute() && transfer.is_none() {
            transfer = Some(*family);
        }
    }

    // Return by queue usage in this order: graphics, present, compute and transfer.
    vec![
        graphics.unwrap(),
        present.unwrap(),
        compute.unwrap(),
        transfer.unwrap(),
    ]
}

#[derive(Debug, Clone, Copy)]
struct QueueFamily {
    index: u32,
    properties: vk::QueueFamilyProperties,
    supports_present: bool,
}

impl QueueFamily {
    fn new(index: u32, properties: vk::QueueFamilyProperties, supports_present: bool) -> Self {
        Self {
            index,
            properties,
            supports_present,
        }
    }

    fn supports_graphics(&self) -> bool {
        self.properties
            .queue_flags
            .contains(vk::QueueFlags::GRAPHICS)
    }

    fn supports_compute(&self) -> bool {
        self.properties
            .queue_flags
            .contains(vk::QueueFlags::COMPUTE)
    }

    fn supports_transfer(&self) -> bool {
        self.properties
            .queue_flags
            .contains(vk::QueueFlags::TRANSFER)
    }

    fn _supports_timestamps(&self) -> bool {
        self.properties.timestamp_valid_bits > 0
    }
}

struct QueueSubmitSemaphoreDescriptor<'a> {
    semaphore: &'a Semaphore,
    stage_mask: vk::PipelineStageFlags2,
    /// Only necessary timeline semaphores.
    value: Option<u64>,
}

#[derive(Clone)]
struct Queue {
    /// Handy for queue submission.
    ash_device: ash::Device,
    raw: vk::Queue,
    _family_index: u32,
}

impl Queue {
    fn new_from_vulkan_handle(ash_device: ash::Device, raw: vk::Queue, family_index: u32) -> Self {
        Self {
            ash_device,
            raw,
            _family_index: family_index,
        }
    }

    fn submit_command_buffers(
        &self,
        command_buffers: &[vk::CommandBuffer],
        wait_semaphores: &[QueueSubmitSemaphoreDescriptor],
        signal_semaphores: &[QueueSubmitSemaphoreDescriptor],
    ) -> Result<()> {
        let wait_semaphores_info = wait_semaphores
            .iter()
            .map(|submit_info| {
                vk::SemaphoreSubmitInfo::default()
                    .semaphore(submit_info.semaphore.raw)
                    .stage_mask(submit_info.stage_mask)
                    .value(
                        if submit_info.semaphore.semaphore_type == SemaphoreType::Timeline {
                            submit_info
                                .value
                                .expect("Timeline semaphore requires a value!")
                        } else {
                            0
                        },
                    )
            })
            .collect::<Vec<_>>();

        let signal_semaphores_info = signal_semaphores
            .iter()
            .map(|submit_info| {
                vk::SemaphoreSubmitInfo::default()
                    .semaphore(submit_info.semaphore.raw)
                    .stage_mask(submit_info.stage_mask)
                    .value(
                        if submit_info.semaphore.semaphore_type == SemaphoreType::Timeline {
                            submit_info
                                .value
                                .expect("Timeline semaphore requires a value!")
                        } else {
                            0
                        },
                    )
            })
            .collect::<Vec<_>>();

        let command_buffer_submit_infos = command_buffers
            .into_iter()
            .map(|command_buffer| {
                vk::CommandBufferSubmitInfo::default().command_buffer(command_buffer.clone())
            })
            .collect::<Vec<_>>();

        let submit_info = vk::SubmitInfo2::default()
            .wait_semaphore_infos(&wait_semaphores_info[..])
            .signal_semaphore_infos(&signal_semaphores_info[..])
            .command_buffer_infos(&command_buffer_submit_infos[..]);

        unsafe {
            self.ash_device.queue_submit2(
                self.raw,
                std::slice::from_ref(&submit_info),
                vk::Fence::null(),
            )?
        };

        Ok(())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum SemaphoreType {
    Binary,
    Timeline,
}

struct Semaphore {
    /// Required to destroy the semaphore inside destructor
    device: Arc<DeviceShared>,
    raw: vk::Semaphore,
    semaphore_type: SemaphoreType,
}

impl Semaphore {
    fn new(device: Arc<DeviceShared>, semaphore_type: SemaphoreType) -> Result<Self> {
        let semaphore_info = vk::SemaphoreCreateInfo::default();

        let mut semaphore_type_info =
            vk::SemaphoreTypeCreateInfo::default().semaphore_type(vk::SemaphoreType::BINARY);
        if semaphore_type == SemaphoreType::Timeline {
            semaphore_type_info = semaphore_type_info.semaphore_type(vk::SemaphoreType::TIMELINE);
        }
        let semaphore_info = semaphore_info.push_next(&mut semaphore_type_info);

        let raw = unsafe { device.raw.create_semaphore(&semaphore_info, None)? };

        Ok(Self {
            device,
            raw,
            semaphore_type,
        })
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
            self.device.raw.destroy_semaphore(self.raw, None);
        }
    }
}
