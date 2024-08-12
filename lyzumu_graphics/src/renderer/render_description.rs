use nalgebra::Vector3;

use crate::vulkan::shader::ShaderModuleDescriptor;

///
/// Describes how the gameplay scene is rendered.
///

pub struct RenderPipelineDescription {
    pub instanced_type: InstancedType,
    pub shader_modules: Vec<ShaderModuleDescriptor>,
}

pub enum InstancedType {
    /// One set of vertices and indices for all instanced objects.
    SingleVertices,
    /// Different vertices and indices with same vertex and index count for all objects.
    DynamicVertices,
}

pub struct InstancedDrawData {
    pub instanced_type: InstancedType,

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
}
