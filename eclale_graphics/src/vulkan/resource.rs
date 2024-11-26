use std::{
    collections::HashMap,
    ffi::CString,
    mem::{align_of, size_of, size_of_val},
    sync::Arc,
};

use anyhow::Result;
use ash::vk::{self, Extent3D};
use gpu_allocator::{
    vulkan::{Allocation, AllocationCreateDesc, AllocationScheme},
    MemoryLocation,
};

use super::types::{
    DescriptorSetLayoutBinding, PipelineDepthStencilState, PipelineRasterizationState,
};

use super::{device::Device, shader::ShaderModule, DeviceShared};

pub struct BufferDescriptor {
    pub size: u64,
    pub usage_flags: vk::BufferUsageFlags,
    pub memory_location: MemoryLocation,
}

impl BufferDescriptor {
    pub fn new(
        size: u64,
        usage_flags: vk::BufferUsageFlags,
        memory_location: MemoryLocation,
    ) -> Self {
        Self {
            size,
            usage_flags,
            memory_location,
        }
    }
}

pub struct Buffer {
    pub(crate) raw: vk::Buffer,
    size: u64,
    allocation: Option<Allocation>,
    device: Arc<Device>,
}

/// Buffer that is pending for actual vulkan destruction.
/// This structure should not hold the actual `Device` resource to prevent circular referencing.
pub(crate) struct PendingDestructionBuffer {
    raw: vk::Buffer,
    allocation: Allocation,
    // Add other info such as frame submission index as required....
}

impl Buffer {
    /// Writes to a CPU->GPU buffer. Returns error if buffer is not writable from the CPU.
    pub fn write_data<T: Copy>(&self, data: &[T]) -> Result<()> {
        unsafe {
            let data_ptr = self
                .allocation
                .as_ref()
                .unwrap()
                .mapped_ptr()
                .unwrap()
                .as_ptr();

            let mut align =
                ash::util::Align::new(data_ptr, align_of::<T>() as _, size_of_val(data) as _);
            align.copy_from_slice(data);
        };

        Ok(())
    }

    pub fn write_data_with_value_offset<T: Copy>(
        &self,
        data: &[T],
        value_offset: u64,
    ) -> Result<()> {
        unsafe {
            let data_ptr = self
                .allocation
                .as_ref()
                .unwrap()
                .mapped_ptr()
                .unwrap()
                .as_ptr()
                .add(value_offset as usize * size_of::<T>());

            let mut align =
                ash::util::Align::new(data_ptr, align_of::<T>() as _, size_of_val(data) as _);
            align.copy_from_slice(data);
        };

        Ok(())
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        let allocation = self.allocation.take().unwrap();
        self.device.schedule_destruction_buffer(self, allocation);
    }
}

pub struct ImageDescriptor {
    pub width: u32,
    pub height: u32,
    pub depth: u32,

    pub array_layer_count: u32,
    pub mip_level_count: u32,

    pub format: vk::Format,
    pub image_type: vk::ImageType,
    pub usage_flags: vk::ImageUsageFlags,

    pub memory_location: MemoryLocation,
}

impl ImageDescriptor {
    pub fn new_2d_single_layer_level(width: u32, height: u32, format: vk::Format) -> Self {
        Self {
            width,
            height,
            depth: 1,
            array_layer_count: 1,
            mip_level_count: 1,
            format,
            image_type: vk::ImageType::TYPE_2D,
            usage_flags: vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            memory_location: MemoryLocation::GpuOnly,
        }
    }
}

pub struct Image {
    pub(crate) raw: vk::Image,
    allocation: Option<Allocation>,
    pub(crate) raw_view: vk::ImageView,

    device: Arc<Device>,

    pub(crate) extent: Extent3D,
    array_layer_count: u32,
    mip_level_count: u32,
    pub format: vk::Format,
}

pub(crate) struct PendingDestructionImage {
    pub(crate) raw: vk::Image,
    pub(crate) raw_view: vk::ImageView,
    pub(crate) allocation: Allocation,
}

impl Drop for Image {
    fn drop(&mut self) {
        let allocation = self.allocation.take().unwrap();
        self.device.schedule_destruction_image(self, allocation);
    }
}

fn vulkan_image_type_to_view_type(image_type: vk::ImageType) -> vk::ImageViewType {
    match image_type {
        vk::ImageType::TYPE_2D => vk::ImageViewType::TYPE_2D,
        _ => {
            todo!()
        }
    }
}

pub struct SamplerDescriptor {
    pub min_filter: vk::Filter,
    pub mag_filter: vk::Filter,
    pub mipmap_mode: vk::SamplerMipmapMode,
    pub address_mode_u: vk::SamplerAddressMode,
    pub address_mode_v: vk::SamplerAddressMode,
    pub address_mode_w: vk::SamplerAddressMode,
    pub reduction_mode: vk::SamplerReductionMode,
}

impl SamplerDescriptor {
    pub fn new() -> Self {
        Self {
            min_filter: vk::Filter::LINEAR,
            mag_filter: vk::Filter::LINEAR,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            address_mode_u: vk::SamplerAddressMode::REPEAT,
            address_mode_v: vk::SamplerAddressMode::REPEAT,
            address_mode_w: vk::SamplerAddressMode::REPEAT,
            reduction_mode: vk::SamplerReductionMode::WEIGHTED_AVERAGE,
        }
    }

    pub fn min_filter(mut self, min_filter: vk::Filter) -> Self {
        self.min_filter = min_filter;
        self
    }

    pub fn mag_filter(mut self, mag_filter: vk::Filter) -> Self {
        self.mag_filter = mag_filter;
        self
    }
}

pub struct Sampler {
    device: Arc<Device>,
    raw: vk::Sampler,
}

pub(crate) struct PendingDestructionSampler {
    raw: vk::Sampler,
}

impl Drop for Sampler {
    fn drop(&mut self) {
        self.device
            .schedule_destruction_sampler(PendingDestructionSampler { raw: self.raw });
    }
}

pub struct PipelineDescriptor {
    /// vkPipelineLayoutCreateInfo information. Descriptor binding layout is required.
    pub descriptor_set_layouts: Vec<Arc<DescriptorSetLayout>>,

    /// vkPipelineCreateInfo information.
    pub shader_modules: Vec<ShaderModule>,
    pub vertex_input_attributes: Vec<vk::VertexInputAttributeDescription>,
    pub vertex_input_bindings: Vec<vk::VertexInputBindingDescription>,
    pub primitive_topology: vk::PrimitiveTopology,
    pub viewport_scissor_extent: vk::Extent2D,
    pub color_blend_attachments: Vec<vk::PipelineColorBlendAttachmentState>, // Should be equal to the number of color attachments.
    pub depth_stencil_state: PipelineDepthStencilState,
    pub rasterization_state: PipelineRasterizationState,

    /// Required for dynamic rendering.
    pub color_attachment_formats: Vec<vk::Format>,
    pub depth_attachment_format: vk::Format,
}

pub struct Pipeline {
    pub(crate) raw: vk::Pipeline,
    pub(crate) raw_layout: vk::PipelineLayout,

    /// XXX: Do we need to hold onto the descriptor set layouts after the pipelin layout is created?
    _descriptor_set_layouts: Vec<Arc<DescriptorSetLayout>>,
    device: Arc<Device>,
}

pub(crate) struct PendingDestructionPipeline {
    raw: vk::Pipeline,
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            self.device
                .shared
                .raw
                .destroy_pipeline_layout(self.raw_layout, None);
            self.device.schedule_destruction_pipeline(self);
        }
    }
}

/// Small wrapper around `vkDescriptorPool`.
pub(crate) struct DescriptorPool {
    raw: vk::DescriptorPool,
    device: Arc<DeviceShared>,
}

impl DescriptorPool {
    pub(crate) fn new(
        device: Arc<DeviceShared>,
        desc: vk::DescriptorPoolCreateInfo,
    ) -> Result<Self> {
        let raw = unsafe { device.raw.create_descriptor_pool(&desc, None)? };

        Ok(Self { raw, device })
    }
}

impl Drop for DescriptorPool {
    fn drop(&mut self) {
        unsafe {
            self.device.raw.destroy_descriptor_pool(self.raw, None);
        }
    }
}

pub struct DescriptorSetLayoutDescriptor {
    pub bindings: Vec<DescriptorSetLayoutBinding>,
    pub flags: vk::DescriptorSetLayoutCreateFlags,
    pub binding_flags: Option<Vec<vk::DescriptorBindingFlags>>,
}

impl DescriptorSetLayoutDescriptor {
    pub fn new(
        bindings: Vec<DescriptorSetLayoutBinding>,
        flags: vk::DescriptorSetLayoutCreateFlags,
    ) -> Self {
        Self {
            bindings,
            flags,
            binding_flags: None,
        }
    }

    pub fn new_with_binding_flags(
        bindings: Vec<DescriptorSetLayoutBinding>,
        flags: vk::DescriptorSetLayoutCreateFlags,
        binding_flags: Vec<vk::DescriptorBindingFlags>,
    ) -> Self {
        Self {
            bindings,
            flags,
            binding_flags: Some(binding_flags),
        }
    }

    pub fn new_with_update_after_bind_flags(
        bindings: Vec<DescriptorSetLayoutBinding>,
        mut flags: vk::DescriptorSetLayoutCreateFlags,
    ) -> Self {
        flags |= vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL;
        let binding_flags = vec![
            vk::DescriptorBindingFlags::PARTIALLY_BOUND
                        // | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT
                        | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND;
            bindings.len()
        ];
        Self {
            bindings,
            flags,
            binding_flags: Some(binding_flags),
        }
    }
}

pub struct DescriptorSetLayout {
    raw: vk::DescriptorSetLayout,
    _bindings: Vec<DescriptorSetLayoutBinding>,
    bindings_map: HashMap<u32, DescriptorSetLayoutBinding>,
    device: Arc<DeviceShared>,
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        unsafe {
            self.device
                .raw
                .destroy_descriptor_set_layout(self.raw, None);
        }
    }
}

#[derive(Clone)]
pub enum DescriptorSetPoolType {
    GlobalGenericResource,
    BindlessTextures,
}

#[derive(Clone)]
pub struct DescriptorSetDescriptor {
    pub layout: Arc<DescriptorSetLayout>,
    pub pool_type: DescriptorSetPoolType,
}

impl DescriptorSetDescriptor {
    pub fn new(layout: Arc<DescriptorSetLayout>, pool_type: DescriptorSetPoolType) -> Self {
        Self { layout, pool_type }
    }

    pub fn new_generic(layout: Arc<DescriptorSetLayout>) -> Self {
        Self::new(layout, DescriptorSetPoolType::GlobalGenericResource)
    }
}

pub struct DescriptorSet {
    pub(crate) raw: vk::DescriptorSet,

    /// Do not need to hold the pool object itself as the global pool is tied to `Device`,
    /// and when `Device` is dropped this descriptor set object cannot be used anymore anyways.
    ///
    /// XXX: Need to hold onto the resource bindings as well(eg. buffers and images)?
    layout: Arc<DescriptorSetLayout>,
    _device: Arc<DeviceShared>,
}

/// XXX: The descriptor set is tehcnically responsible for keeping its bounded reosurces valid.
/// Maybe hold a strong reference to the bounded resources as well?
#[derive(Clone)]
pub struct DescriptorBindingBufferWrite<'a> {
    pub buffer: &'a Buffer,
    pub binding_index: u32,
}

#[derive(Clone)]
pub struct DescriptorBindingWrites<'a> {
    pub buffers: Vec<DescriptorBindingBufferWrite<'a>>,
}

#[derive(Clone)]
pub struct DescriptorBindingImageSamplerWrite<'a> {
    pub binding_index: u32,
    pub array_element: u32,
    pub image: &'a Image,
    pub sampler: &'a Sampler,
}

pub struct DescriptorBindingBindlessWrites<'a> {
    pub images: Vec<DescriptorBindingImageSamplerWrite<'a>>,
}

fn format_has_depth(format: vk::Format) -> bool {
    match format {
        vk::Format::D32_SFLOAT_S8_UINT
        | vk::Format::D32_SFLOAT
        | vk::Format::D24_UNORM_S8_UINT
        | vk::Format::D16_UNORM_S8_UINT => true,
        _ => false,
    }
}

fn format_has_stencil(format: vk::Format) -> bool {
    match format {
        vk::Format::D32_SFLOAT_S8_UINT
        | vk::Format::D24_UNORM_S8_UINT
        | vk::Format::D16_UNORM_S8_UINT => true,
        _ => false,
    }
}

impl Device {
    pub fn create_buffer(self: &Arc<Self>, desc: BufferDescriptor) -> Result<Buffer> {
        let create_info = vk::BufferCreateInfo::default().size(desc.size).usage(
            desc.usage_flags
                | vk::BufferUsageFlags::TRANSFER_SRC
                | vk::BufferUsageFlags::TRANSFER_DST,
        );

        let raw;
        let requirements;
        unsafe {
            raw = self.shared.raw.create_buffer(&create_info, None)?;
            requirements = self.shared.raw.get_buffer_memory_requirements(raw);
        }

        let allocation = self
            .shared
            .allocator
            .lock()
            .allocate(&AllocationCreateDesc {
                name: "buffer",
                requirements,
                location: desc.memory_location,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })?;

        unsafe {
            self.shared
                .raw
                .bind_buffer_memory(raw, allocation.memory(), allocation.offset())?;
        }

        Ok(Buffer {
            device: self.clone(),
            raw,
            size: desc.size,
            allocation: Some(allocation),
        })
    }

    /// Schedules/queues a buffer for destruction. `buffer` should no longer be used after this is called
    /// but it is passed in as a reference so this can be called inside `drop`.
    fn schedule_destruction_buffer(&self, buffer: &Buffer, allocation: Allocation) {
        self.resource_hub
            .lock()
            .pending_destruction_buffers
            .push(PendingDestructionBuffer {
                raw: buffer.raw,
                allocation,
            })
    }

    /// Destroys and deallocate buffer GPU resources.
    pub(crate) fn destroy_buffer(&self, buffer: PendingDestructionBuffer) -> Result<()> {
        unsafe {
            self.shared.raw.destroy_buffer(buffer.raw, None);
            self.shared.allocator.lock().free(buffer.allocation)?;
        }

        Ok(())
    }

    pub fn create_image(self: &Arc<Self>, desc: ImageDescriptor) -> Result<Image> {
        let usage_flags = desc.usage_flags
            | vk::ImageUsageFlags::TRANSFER_SRC
            | vk::ImageUsageFlags::TRANSFER_DST;
        let extent = vk::Extent3D {
            width: desc.width,
            height: desc.height,
            depth: desc.depth,
        };

        let create_info = vk::ImageCreateInfo::default()
            .image_type(desc.image_type)
            .format(desc.format)
            .extent(extent)
            .mip_levels(desc.mip_level_count)
            .array_layers(desc.array_layer_count)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(usage_flags)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        let raw = unsafe { self.shared.raw.create_image(&create_info, None)? };
        let requirements = unsafe { self.shared.raw.get_image_memory_requirements(raw) };

        let allocation = self
            .shared
            .allocator
            .lock()
            .allocate(&AllocationCreateDesc {
                name: "image",
                requirements,
                location: desc.memory_location,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })?;
        unsafe {
            self.shared
                .raw
                .bind_image_memory(raw, allocation.memory(), allocation.offset())?;
        };

        // Create ImageView.
        let mut aspect_flags = vk::ImageAspectFlags::empty();
        if format_has_depth(desc.format) {
            aspect_flags |= vk::ImageAspectFlags::DEPTH;
        } else {
            aspect_flags |= vk::ImageAspectFlags::COLOR;
        }
        let subresource_range = vk::ImageSubresourceRange::default()
            .aspect_mask(aspect_flags)
            .base_mip_level(0)
            .level_count(desc.mip_level_count)
            .base_array_layer(0)
            .layer_count(desc.array_layer_count);
        let view_create_info = vk::ImageViewCreateInfo::default()
            .image(raw)
            .view_type(vulkan_image_type_to_view_type(desc.image_type))
            .format(desc.format)
            .subresource_range(subresource_range);
        let raw_view = unsafe { self.shared.raw.create_image_view(&view_create_info, None)? };

        Ok(Image {
            raw,
            allocation: Some(allocation),
            raw_view,
            device: self.clone(),
            extent: vk::Extent3D {
                width: desc.width,
                height: desc.height,
                depth: desc.depth,
            },
            array_layer_count: desc.array_layer_count,
            mip_level_count: desc.mip_level_count,
            format: desc.format,
        })
    }

    fn schedule_destruction_image(self: &Arc<Self>, image: &Image, allocation: Allocation) {
        self.resource_hub
            .lock()
            .pending_destruction_images
            .push(PendingDestructionImage {
                raw: image.raw,
                raw_view: image.raw_view,
                allocation,
            });
    }

    pub(crate) fn destroy_image(&self, image: PendingDestructionImage) -> Result<()> {
        unsafe {
            self.shared.raw.destroy_image(image.raw, None);
            self.shared.raw.destroy_image_view(image.raw_view, None);
            self.shared.allocator.lock().free(image.allocation)?;
        }

        Ok(())
    }

    pub fn create_sampler(self: &Arc<Self>, desc: SamplerDescriptor) -> Result<Sampler> {
        let mut create_info = vk::SamplerCreateInfo::default()
            .min_filter(desc.min_filter)
            .mag_filter(desc.mag_filter)
            .mipmap_mode(desc.mipmap_mode)
            .address_mode_u(desc.address_mode_u)
            .address_mode_v(desc.address_mode_v)
            .address_mode_u(desc.address_mode_u)
            .mip_lod_bias(1.0)
            .anisotropy_enable(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .min_lod(1.0)
            .max_lod(16.0)
            .border_color(vk::BorderColor::INT_OPAQUE_WHITE)
            .unnormalized_coordinates(false);

        let mut sampler_reduction_info = vk::SamplerReductionModeCreateInfo::default();
        if desc.reduction_mode != vk::SamplerReductionMode::WEIGHTED_AVERAGE {
            sampler_reduction_info = sampler_reduction_info.reduction_mode(desc.reduction_mode);
            create_info = create_info.push_next(&mut sampler_reduction_info);
        }

        let raw = unsafe { self.shared.raw.create_sampler(&create_info, None)? };

        Ok(Sampler {
            device: self.clone(),
            raw,
        })
    }

    fn schedule_destruction_sampler(self: &Arc<Self>, sampler: PendingDestructionSampler) {
        self.resource_hub
            .lock()
            .pending_destruction_sampler
            .push(sampler);
    }

    pub(crate) fn destroy_sampler(&self, sampler: PendingDestructionSampler) -> Result<()> {
        unsafe {
            self.shared.raw.destroy_sampler(sampler.raw, None);
        }
        Ok(())
    }

    pub fn create_pipeline(self: &Arc<Self>, desc: PipelineDescriptor) -> Result<Pipeline> {
        let descriptor_set_layouts = desc
            .descriptor_set_layouts
            .iter()
            .map(|layout| layout.raw)
            .collect::<Vec<_>>();
        let pipeline_layout_info =
            vk::PipelineLayoutCreateInfo::default().set_layouts(&descriptor_set_layouts);
        let pipeline_layout = unsafe {
            self.shared
                .raw
                .create_pipeline_layout(&pipeline_layout_info, None)?
        };

        let shader_entry_point_name = CString::new("main").unwrap();
        let shader_stages = desc
            .shader_modules
            .iter()
            .map(|shader_module| {
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(shader_module.stage.to_vulkan_shader_stage_flag())
                    .module(shader_module.raw)
                    .name(&shader_entry_point_name)
            })
            .collect::<Vec<_>>();

        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&desc.vertex_input_attributes)
            .vertex_binding_descriptions(&desc.vertex_input_bindings);

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(desc.primitive_topology)
            .primitive_restart_enable(false);

        let viewports = [vk::Viewport::default()
            .x(0.0)
            .y(0.0)
            .width(desc.viewport_scissor_extent.width as f32)
            .height(desc.viewport_scissor_extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0)];
        let scissors = [vk::Rect2D::default()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(desc.viewport_scissor_extent)];
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewports(&viewports)
            .scissors(&scissors);

        // Individual color blend attachments needs color write mask to be RGBA(?).
        // Need one color blend attachment state for each color attachement(render target).
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(&desc.color_blend_attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0]);

        let multisample_state = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .sample_shading_enable(false)
            .min_sample_shading(1.0);

        let dynamic_states = vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let mut pipeline_rendering_info = vk::PipelineRenderingCreateInfo::default()
            .view_mask(0)
            .color_attachment_formats(&desc.color_attachment_formats)
            .depth_attachment_format(desc.depth_attachment_format)
            .stencil_attachment_format(vk::Format::UNDEFINED);

        let vulkan_depth_stencil_state = desc.depth_stencil_state.to_vulkan_state();
        let vulkan_rasterization_state = desc.rasterization_state.to_vulkan_state();

        let pipeline_create_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state)
            .color_blend_state(&color_blend_state)
            .depth_stencil_state(&vulkan_depth_stencil_state)
            .multisample_state(&multisample_state)
            .rasterization_state(&vulkan_rasterization_state)
            .dynamic_state(&dynamic_state)
            .layout(pipeline_layout)
            .push_next(&mut pipeline_rendering_info);

        let raw = unsafe {
            self.shared
                .raw
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    std::slice::from_ref(&pipeline_create_info),
                    None,
                )
                .map_err(|e| e.1)?[0]
        };

        Ok(Pipeline {
            raw,
            raw_layout: pipeline_layout,
            _descriptor_set_layouts: desc.descriptor_set_layouts,
            device: self.clone(),
        })
    }

    fn schedule_destruction_pipeline(&self, pipeline: &Pipeline) {
        self.resource_hub
            .lock()
            .pending_destruction_pipelines
            .push(PendingDestructionPipeline { raw: pipeline.raw });
    }

    pub(crate) fn destroy_pipeline(&self, pipeline: PendingDestructionPipeline) -> Result<()> {
        unsafe {
            self.shared.raw.destroy_pipeline(pipeline.raw, None);
        }

        Ok(())
    }

    pub fn create_descriptor_set_layout(
        &self,
        desc: DescriptorSetLayoutDescriptor,
    ) -> Result<DescriptorSetLayout> {
        let vulkan_descriptor_bindings = desc
            .bindings
            .iter()
            .map(|b| b.to_vulkan_binding())
            .collect::<Vec<_>>();

        let mut create_info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&vulkan_descriptor_bindings)
            .flags(desc.flags);
        let mut binding_flags_info = vk::DescriptorSetLayoutBindingFlagsCreateInfo::default();

        if let Some(binding_flags) = &desc.binding_flags {
            assert_eq!(desc.bindings.len(), binding_flags.len());

            binding_flags_info = binding_flags_info.binding_flags(&binding_flags);
            create_info = create_info.push_next(&mut binding_flags_info)
        };

        let raw = unsafe {
            self.shared
                .raw
                .create_descriptor_set_layout(&create_info, None)?
        };

        let bindings_map = desc
            .bindings
            .iter()
            .cloned()
            .map(|binding| (binding.binding, binding))
            .collect();

        Ok(DescriptorSetLayout {
            raw,
            _bindings: desc.bindings,
            bindings_map,
            device: self.shared.clone(),
        })
    }

    pub fn create_descriptor_set(&self, desc: DescriptorSetDescriptor) -> Result<DescriptorSet> {
        let pool_raw = match desc.pool_type {
            DescriptorSetPoolType::GlobalGenericResource => self.global_descriptor_pool.raw,
            DescriptorSetPoolType::BindlessTextures => {
                self.global_descriptor_pool_bindless_textures.raw
            }
        };
        let allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(pool_raw)
            .set_layouts(std::slice::from_ref(&desc.layout.raw));
        let raws = unsafe { self.shared.raw.allocate_descriptor_sets(&allocate_info)? };

        Ok(DescriptorSet {
            raw: raws[0],
            layout: desc.layout,
            _device: self.shared.clone(),
        })
    }

    /// Binds descriptor set with resource writes.
    pub fn update_descriptor_set(
        &self,
        descriptor_set: &DescriptorSet,
        writes: &DescriptorBindingWrites,
    ) -> Result<()> {
        let mut vulkan_write_descriptors = Vec::new();

        // Image/buffer descriptor write infos need to be valid when calling vkUpdateDescriptorSets.
        // Write structures need to be updated and referenced to this array before calling vkUpdateDescriptorSets.
        // Second element is index to `vulkan_write_descriptors`.
        let mut descriptor_buffer_infos = Vec::<(vk::DescriptorBufferInfo, usize)>::new();

        for buffer_write in &writes.buffers {
            if let Some(binding) = descriptor_set
                .layout
                .bindings_map
                .get(&buffer_write.binding_index)
            {
                assert_eq!(
                    binding.binding, buffer_write.binding_index,
                    "Descriptor set layout binding index and buffer write binding do not match."
                );

                let vulkan_write_descriptor = vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set.raw)
                    .dst_binding(binding.binding)
                    .dst_array_element(0)
                    .descriptor_type(binding.descriptor_type);

                match binding.descriptor_type {
                    vk::DescriptorType::UNIFORM_BUFFER | vk::DescriptorType::STORAGE_BUFFER => {
                        let vulkan_buffer_info = vk::DescriptorBufferInfo::default()
                            .offset(0)
                            .range(buffer_write.buffer.size as u64)
                            .buffer(buffer_write.buffer.raw);
                        descriptor_buffer_infos
                            .push((vulkan_buffer_info, vulkan_write_descriptors.len()));

                        vulkan_write_descriptors.push(vulkan_write_descriptor);
                    }
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Cannot handle descriptor type {:#?}",
                            binding.descriptor_type
                        ));
                    }
                }
            } else {
                return Err(anyhow::anyhow!(
                    "Binding index {} on descriptor buffer write is invalid!",
                    buffer_write.binding_index
                ));
            }
        }

        // Update descriptor resource infos for write structures.
        for (buffer_info, write_index) in &descriptor_buffer_infos {
            vulkan_write_descriptors[*write_index] = vulkan_write_descriptors[*write_index]
                .buffer_info(std::slice::from_ref(buffer_info));
        }

        unsafe {
            self.shared
                .raw
                .update_descriptor_sets(&vulkan_write_descriptors, &[]);
        }

        Ok(())
    }

    pub fn update_descriptor_set_bindless(
        &self,
        descriptor_set: &DescriptorSet,
        writes: &DescriptorBindingBindlessWrites,
    ) -> Result<()> {
        let mut vulkan_write_descriptors = Vec::new();

        // Image/buffer descriptor write infos need to be valid when calling vkUpdateDescriptorSets.
        // Write structures need to be updated and referenced to this array before calling vkUpdateDescriptorSets.
        // Second element is index to `vulkan_write_descriptors`.
        let mut descriptor_image_infos = Vec::<(vk::DescriptorImageInfo, usize)>::new();

        for image_write in &writes.images {
            if let Some(binding) = descriptor_set
                .layout
                .bindings_map
                .get(&image_write.binding_index)
            {
                assert_eq!(
                    binding.binding, image_write.binding_index,
                    "Descriptor set bindless layout binding index and buffer write binding do not match."
                );

                let vulkan_write_descriptor = vk::WriteDescriptorSet::default()
                    .dst_set(descriptor_set.raw)
                    .dst_binding(binding.binding)
                    .dst_array_element(image_write.array_element)
                    .descriptor_type(binding.descriptor_type);

                match binding.descriptor_type {
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER => {
                        let vulkan_image_info = vk::DescriptorImageInfo::default()
                            .image_view(image_write.image.raw_view)
                            .sampler(image_write.sampler.raw)
                            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
                        descriptor_image_infos
                            .push((vulkan_image_info, vulkan_write_descriptors.len()));

                        vulkan_write_descriptors.push(vulkan_write_descriptor);
                    }
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Cannot handle descriptor type {:#?}",
                            binding.descriptor_type
                        ));
                    }
                }
            } else {
                return Err(anyhow::anyhow!(
                    "Binding index {} on descriptor buffer write is invalid!",
                    image_write.binding_index
                ));
            }
        }

        // Update descriptor resource infos for write structures.
        // XXX: Does this have to be done outside of the loop above? can probably just pre-allocate
        // resource infos.
        for (image_info, write_index) in &descriptor_image_infos {
            vulkan_write_descriptors[*write_index] =
                vulkan_write_descriptors[*write_index].image_info(std::slice::from_ref(image_info))
        }

        unsafe {
            self.shared
                .raw
                .update_descriptor_sets(&vulkan_write_descriptors, &[]);
        }

        Ok(())
    }
}
