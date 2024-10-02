use nalgebra::Vector3;

use crate::vulkan::shader::ShaderModuleDescriptor;

///
/// Describes how the gameplay scene is rendered.
///

pub enum RenderingType {
    Instanced,
}

pub struct RenderPipelineDescription {
    pub rendering_type: RenderingType,
    pub shader_modules: Vec<ShaderModuleDescriptor>,
}

pub struct InstancedDrawData {
    pub rendering_type: RenderingType,

    pub instance_data: Vec<u8>,
    pub instance_count: usize,

    pub vertices: Vec<Vector3<f32>>,
    pub indices: Vec<u16>,

    /// Index to render description `pipelines` array.
    pub pipeline_index: usize,
}

pub struct RenderDescription {
    pub pipelines: Vec<RenderPipelineDescription>,
    pub instanced_draw_data: Vec<InstancedDrawData>,
    pub scene_uniform_data_size: u64,
}
