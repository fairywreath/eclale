use anyhow::Result;

use nalgebra::{Isometry3, Matrix4, Orthographic3, Perspective3, Point3, Vector3, Vector4};
use winit::raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use lyzumu_graphics::{
    mesh::polyhedron::Polyhedron,
    renderer::{
        render_description::{
            InstancedDrawData, InstancedType, RenderDescription, RenderPipelineDescription,
        },
        Renderer,
    },
    vulkan::shader::{ShaderModuleDescriptor, ShaderStage},
};

#[derive(Clone, Copy)]
struct HitObjectInstanceData {
    transform: Matrix4<f32>,
    base_color: Vector4<f32>,
}

#[derive(Clone, Copy, Default)]
struct SceneUniformData {
    view_projection: Matrix4<f32>,
    runner_transform: Matrix4<f32>,
}

fn create_render_description() -> RenderDescription {
    let hit_object_pipeline_description = RenderPipelineDescription {
        instanced_type: InstancedType::SingleVertices,
        shader_modules: vec![
            ShaderModuleDescriptor {
                source_file_name: String::from("shaders/hit_object.vs.glsl"),
                shader_stage: ShaderStage::Vertex,
            },
            ShaderModuleDescriptor {
                source_file_name: String::from("shaders/hit_object.fs.glsl"),
                shader_stage: ShaderStage::Fragment,
            },
        ],
    };

    let hit_object_mesh = Polyhedron::cuboid(0.4, 0.1, 0.2);
    let hit_object_draw_data = InstancedDrawData {
        instanced_type: InstancedType::SingleVertices,

        // XXX TODO:
        instance_data: Vec::new(),
        instance_count: 0,

        vertices: hit_object_mesh.vertices.clone(),
        indices: hit_object_mesh.indices.clone(),
        pipeline_index: 0,
    };

    RenderDescription {
        pipelines: vec![hit_object_pipeline_description],
        instanced_draw_data: vec![hit_object_draw_data],
    }
}

pub(crate) struct TrackRenderer {
    renderer: Renderer,
}

impl TrackRenderer {
    pub(crate) fn new(
        window_handle: RawWindowHandle,
        display_handle: RawDisplayHandle,
    ) -> Result<Self> {
        todo!()
    }
}
