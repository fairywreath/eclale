use nalgebra::{Vector2, Vector3};

use crate::{geometry::line::Line, vulkan::shader::ShaderModuleDescriptor};

#[derive(Clone, Copy)]
pub enum RenderingType {
    /// Instanced objects with per-instance data.
    Instanced,
    /// Multiple different objects under the same vertex bufffers, with per-object data.
    MultipleObjectsSingleVertexData,
}

#[derive(Clone)]
pub struct RenderPipelineDescription {
    pub rendering_type: RenderingType,
    pub shader_modules: Vec<ShaderModuleDescriptor>,
}

#[derive(Clone)]
pub struct InstancedDrawData {
    pub instance_data: Vec<u8>,
    pub instance_count: usize,

    pub vertices: Vec<Vector3<f32>>,
    pub indices: Vec<u16>,

    /// Index to render description `pipelines` array.
    pub pipeline_index: usize,
}

/// Multiple objects, single vertex attributes, draw data.
#[derive(Clone)]
pub struct MOSVDrawData {
    pub objects_count: usize,
    pub objects_data: Vec<u8>,
    pub objects_indices: Vec<u8>,

    pub vertices: Vec<Vector3<f32>>,
    pub indices: Vec<u16>,

    /// Index to render description `pipelines` array.
    pub pipeline_index: usize,
}

// #[derive(Clone)]
// pub struct LinesDrawData {
//     pub lines: Vec<Line>,
//     pub line_data: Vec<u8>,
//     pub instance_data: Vec<u8>,
//     pub instance_count: usize,
//     pub pipeline_index: usize,
// }

#[derive(Clone)]
pub struct RenderDescription {
    pub scene_uniform_data_size: u64,
    pub pipelines: Vec<RenderPipelineDescription>,

    pub instanced_draw_data: Vec<InstancedDrawData>,
    pub mosv_draw_data: Vec<MOSVDrawData>,
    // pub lines_draw_data: Vec<LinesDrawData>,
}
