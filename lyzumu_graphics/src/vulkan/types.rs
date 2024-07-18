/*! Contains types/structures to be used with the vulkan GPU wrapper API.
 * Raw vulkan structures are used as much as possible, hence this file should mostly contain utility functions to
 * create the vulkan structures.
 */

use ash::vk;

#[derive(Clone)]
pub struct DescriptorSetLayoutBinding {
    pub binding: u32,
    pub descriptor_type: vk::DescriptorType,
    pub descriptor_count: u32,
    pub stage_flags: vk::ShaderStageFlags,
}

impl DescriptorSetLayoutBinding {
    pub fn new() -> Self {
        Self {
            binding: 0,
            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 0,
            stage_flags: vk::ShaderStageFlags::empty(),
        }
    }

    pub fn binding(mut self, binding: u32) -> Self {
        self.binding = binding;
        self
    }

    pub fn descriptor_type(mut self, descriptor_type: vk::DescriptorType) -> Self {
        self.descriptor_type = descriptor_type;
        self
    }

    pub fn descriptor_count(mut self, descriptor_count: u32) -> Self {
        self.descriptor_count = descriptor_count;
        self
    }

    pub fn stage_flags(mut self, stage_flags: vk::ShaderStageFlags) -> Self {
        self.stage_flags = stage_flags;
        self
    }

    pub(crate) fn to_vulkan_binding(&self) -> vk::DescriptorSetLayoutBinding {
        vk::DescriptorSetLayoutBinding::default()
            .binding(self.binding)
            .descriptor_type(self.descriptor_type)
            .descriptor_count(self.descriptor_count)
            .stage_flags(self.stage_flags)
    }
}

pub struct PipelineDepthStencilState {
    pub flags: vk::PipelineDepthStencilStateCreateFlags,
    pub depth_test_enable: bool,
    pub depth_write_enable: bool,
    pub depth_compare_op: vk::CompareOp,
    pub depth_bounds_test_enable: bool,
    pub stencil_test_enable: bool,
    pub front: vk::StencilOpState,
    pub back: vk::StencilOpState,
    pub min_depth_bounds: f32,
    pub max_depth_bounds: f32,
}

impl PipelineDepthStencilState {
    pub fn new() -> Self {
        Self {
            flags: vk::PipelineDepthStencilStateCreateFlags::default(),
            depth_test_enable: false,
            depth_write_enable: false,
            depth_compare_op: vk::CompareOp::default(),
            depth_bounds_test_enable: false,
            stencil_test_enable: false,
            front: vk::StencilOpState::default(),
            back: vk::StencilOpState::default(),
            min_depth_bounds: 0.0,
            max_depth_bounds: 1.0,
        }
    }

    pub fn flags(mut self, flags: vk::PipelineDepthStencilStateCreateFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn depth_test_enable(mut self, depth_test_enable: bool) -> Self {
        self.depth_test_enable = depth_test_enable;
        self
    }

    pub fn depth_write_enable(mut self, depth_write_enable: bool) -> Self {
        self.depth_write_enable = depth_write_enable;
        self
    }

    pub fn depth_compare_op(mut self, depth_compare_op: vk::CompareOp) -> Self {
        self.depth_compare_op = depth_compare_op;
        self
    }

    pub fn depth_bounds_test_enable(mut self, depth_bounds_test_enable: bool) -> Self {
        self.depth_bounds_test_enable = depth_bounds_test_enable;
        self
    }

    pub fn stencil_test_enable(mut self, stencil_test_enable: bool) -> Self {
        self.stencil_test_enable = stencil_test_enable;
        self
    }

    pub fn front(mut self, front: vk::StencilOpState) -> Self {
        self.front = front;
        self
    }

    pub fn back(mut self, back: vk::StencilOpState) -> Self {
        self.back = back;
        self
    }

    pub fn min_depth_bounds(mut self, min_depth_bounds: f32) -> Self {
        self.min_depth_bounds = min_depth_bounds;
        self
    }

    pub fn max_depth_bounds(mut self, max_depth_bounds: f32) -> Self {
        self.max_depth_bounds = max_depth_bounds;
        self
    }

    pub(crate) fn to_vulkan_state(&self) -> vk::PipelineDepthStencilStateCreateInfo {
        vk::PipelineDepthStencilStateCreateInfo::default()
            .flags(self.flags)
            .depth_test_enable(self.depth_test_enable)
            .depth_write_enable(self.depth_write_enable)
            .depth_compare_op(self.depth_compare_op)
            .depth_bounds_test_enable(self.depth_bounds_test_enable)
            .stencil_test_enable(self.stencil_test_enable)
            .front(self.front)
            .back(self.back)
            .min_depth_bounds(self.min_depth_bounds)
            .max_depth_bounds(self.max_depth_bounds)
    }
}

pub struct PipelineRasterizationState {
    pub flags: vk::PipelineRasterizationStateCreateFlags,
    pub depth_clamp_enable: bool,
    pub rasterizer_discard_enable: bool,
    pub polygon_mode: vk::PolygonMode,
    pub cull_mode: vk::CullModeFlags,
    pub front_face: vk::FrontFace,
    pub depth_bias_enable: bool,
    pub depth_bias_constant_factor: f32,
    pub depth_bias_clamp: f32,
    pub depth_bias_slope_factor: f32,
    pub line_width: f32,
}

impl PipelineRasterizationState {
    pub fn new() -> Self {
        Self {
            flags: vk::PipelineRasterizationStateCreateFlags::default(),
            depth_clamp_enable: false,
            rasterizer_discard_enable: false,
            polygon_mode: vk::PolygonMode::FILL,
            cull_mode: vk::CullModeFlags::NONE,
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            depth_bias_enable: false,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
        }
    }

    pub fn flags(mut self, flags: vk::PipelineRasterizationStateCreateFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn depth_clamp_enable(mut self, depth_clamp_enable: bool) -> Self {
        self.depth_clamp_enable = depth_clamp_enable;
        self
    }

    pub fn rasterizer_discard_enable(mut self, rasterizer_discard_enable: bool) -> Self {
        self.rasterizer_discard_enable = rasterizer_discard_enable;
        self
    }

    pub fn polygon_mode(mut self, polygon_mode: vk::PolygonMode) -> Self {
        self.polygon_mode = polygon_mode;
        self
    }

    pub fn cull_mode(mut self, cull_mode: vk::CullModeFlags) -> Self {
        self.cull_mode = cull_mode;
        self
    }

    pub fn front_face(mut self, front_face: vk::FrontFace) -> Self {
        self.front_face = front_face;
        self
    }

    pub fn depth_bias_enable(mut self, depth_bias_enable: bool) -> Self {
        self.depth_bias_enable = depth_bias_enable;
        self
    }

    pub fn depth_bias_constant_factor(mut self, depth_bias_constant_factor: f32) -> Self {
        self.depth_bias_constant_factor = depth_bias_constant_factor;
        self
    }

    pub fn depth_bias_clamp(mut self, depth_bias_clamp: f32) -> Self {
        self.depth_bias_clamp = depth_bias_clamp;
        self
    }

    pub fn depth_bias_slope_factor(mut self, depth_bias_slope_factor: f32) -> Self {
        self.depth_bias_slope_factor = depth_bias_slope_factor;
        self
    }

    pub fn line_width(mut self, line_width: f32) -> Self {
        self.line_width = line_width;
        self
    }

    pub(crate) fn to_vulkan_state(&self) -> vk::PipelineRasterizationStateCreateInfo {
        vk::PipelineRasterizationStateCreateInfo::default()
            .flags(self.flags)
            .depth_clamp_enable(self.depth_clamp_enable)
            .rasterizer_discard_enable(self.rasterizer_discard_enable)
            .polygon_mode(self.polygon_mode)
            .cull_mode(self.cull_mode)
            .front_face(self.front_face)
            .depth_bias_enable(self.depth_bias_enable)
            .depth_bias_constant_factor(self.depth_bias_constant_factor)
            .depth_bias_clamp(self.depth_bias_clamp)
            .depth_bias_slope_factor(self.depth_bias_slope_factor)
            .line_width(self.line_width)
    }
}
