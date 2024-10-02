use anyhow::Result;

use bytemuck::{Pod, Zeroable};
use nalgebra::{
    Isometry3, Matrix4, Orthographic3, Perspective3, Point3, Vector2, Vector3, Vector4,
};
use winit::raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use lyzumu_chart::{
    BasicNoteType, BezierControlPoint, Chart, EvadeNoteType, HoldNote, Note, NoteData, Platform,
};
use lyzumu_graphics::{
    mesh::{plane::Plane, polyhedron::Polyhedron, Mesh},
    renderer::{
        render_description::{
            self, InstancedDrawData, RenderDescription, RenderPipelineDescription, RenderingType,
        },
        Renderer,
    },
    vulkan::shader::{ShaderModuleDescriptor, ShaderStage},
};

#[derive(Clone, Debug)]
pub(crate) struct TrackSettings {
    pub(crate) runner_speed: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
struct SceneUniformGpuData {
    view_projection: Matrix4<f32>,
    runner_transform: Matrix4<f32>,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ObjectInstanceGpuData {
    transform: Matrix4<f32>,
    base_color: Vector4<f32>,
}

impl ObjectInstanceGpuData {
    fn new_from_note_instance(instance: &NoteInstance, track_settings: &TrackSettings) -> Self {
        Self {
            transform: Matrix4::new_translation(&Vector3::new(
                instance.x_position,
                0.0,
                instance.z_position * track_settings.runner_speed,
            )),
            base_color: instance.base_color,
        }
    }

    fn new_from_platform_instance(
        instance: &PlatformInstance,
        track_settings: &TrackSettings,
    ) -> Self {
        Self {
            transform: Matrix4::new_translation(&Vector3::new(
                0.0,
                0.0,
                instance.z_start_position * track_settings.runner_speed,
            )),
            base_color: instance.base_color,
        }
    }
}

#[derive(Clone, Debug)]
struct NoteInstance {
    /// Depth within the lane, with higher values indicating a greater distance.
    z_position: f32,
    z_end_position: f32,
    /// Horizontal position across the lane, from the player's perspective.
    x_position: f32,
    base_color: Vector4<f32>,
}

#[derive(Clone, Debug)]
struct PlatformInstance {
    z_start_position: f32,
    z_end_position: f32,
    base_color: Vector4<f32>,
}

fn get_basic_note_base_color(basic_type: BasicNoteType) -> Vector4<f32> {
    match basic_type {
        BasicNoteType::Basic1 => Vector4::new(1.0, 0.0, 0.0, 1.0),
        BasicNoteType::Basic2 => Vector4::new(0.0, 1.0, 0.0, 1.0),
        BasicNoteType::Basic3 => Vector4::new(0.0, 0.0, 1.0, 1.0),
        BasicNoteType::Basic4 => Vector4::new(1.0, 1.0, 0.0, 1.0),
    }
}

fn get_note_base_color(note: &NoteData) -> Vector4<f32> {
    match note {
        NoteData::Basic(basic_type) => get_basic_note_base_color(*basic_type),
        NoteData::BasicHold((basic_type, _)) => get_basic_note_base_color(*basic_type),
        NoteData::Target => Vector4::new(1.0, 0.0, 1.0, 1.0),
        NoteData::TargetHold(_) => Vector4::new(1.0, 0.0, 1.0, 1.0),
        NoteData::Evade(evade_type) => match evade_type {
            _ => Vector4::new(0.5, 0.5, 0.5, 1.0),
        },
        NoteData::Contact(contact_type) => match contact_type {
            _ => Vector4::new(0.5, 0.9, 0.3, 1.0),
        },
        NoteData::Floor => Vector4::new(0.7, 0.1, 0.7, 1.0),
        NoteData::FloorHold(_) => Vector4::new(0.7, 0.1, 0.7, 1.0),
        NoteData::Flick(_) => Vector4::new(0.7, 0.9, 0.7, 1.0),
    }
}

impl From<&Note> for NoteInstance {
    fn from(note: &Note) -> Self {
        let base_color = get_note_base_color(&note.data);
        let x_position = note.x_position;
        let z_position = note.time.seconds.unwrap();
        let z_end_position = match &note.data {
            NoteData::BasicHold((_, hold_note)) => hold_note.end_time.seconds.unwrap(),
            NoteData::TargetHold(hold_note) => hold_note.end_time.seconds.unwrap(),
            NoteData::FloorHold(hold_note) => hold_note.end_time.seconds.unwrap(),
            _ => z_position,
        };

        Self {
            // XXX TODO: Have proper type-state representation for the seconds field.
            base_color,
            x_position,
            z_position,
            z_end_position,
        }
    }
}

fn note_instances_to_gpu_data_bytes(
    notes: &[NoteInstance],
    track_settings: &TrackSettings,
) -> Vec<u8> {
    notes
        .iter()
        .map(|o| {
            bytemuck::bytes_of(&ObjectInstanceGpuData::new_from_note_instance(
                o,
                track_settings,
            ))
            .to_vec()
        })
        .flatten()
        .collect()
}

fn platform_instances_to_gpu_data_bytes(
    platforms: &[PlatformInstance],
    track_settings: &TrackSettings,
) -> Vec<u8> {
    platforms
        .iter()
        .map(|o| {
            bytemuck::bytes_of(&ObjectInstanceGpuData::new_from_platform_instance(
                o,
                track_settings,
            ))
            .to_vec()
        })
        .flatten()
        .collect()
}

impl From<&Platform> for PlatformInstance {
    fn from(platform: &Platform) -> Self {
        let base_color = Vector4::new(0.0, 0.0, 0.0, 1.0);
        Self {
            z_start_position: platform.start_time.seconds.unwrap(),
            z_end_position: platform.end_time.seconds.unwrap(),
            base_color,
        }
    }
}

struct RenderObjects {
    notes_basic: Vec<NoteInstance>,
    notes_target: Vec<NoteInstance>,
    notes_floor: Vec<NoteInstance>,

    notes_evade: Vec<NoteInstance>,
    notes_contact: Vec<NoteInstance>,

    notes_basic_hold: Vec<NoteInstance>,
    notes_target_hold: Vec<NoteInstance>,
    notes_floor_hold: Vec<NoteInstance>,

    notes_flick: Vec<NoteInstance>,

    platform_instances: Vec<PlatformInstance>,
    platforms_mesh: Mesh,
}

fn filter_and_map_notes<F>(notes: &[Note], predicate: F) -> Vec<NoteInstance>
where
    F: Fn(&Note) -> bool,
{
    notes
        .iter()
        .filter(|n| predicate(n))
        .map(NoteInstance::from)
        .collect()
}

impl RenderObjects {
    fn new_from_chart(chart: &Chart) -> Self {
        // XXX: Just iterate the notes array once.
        let notes_basic =
            filter_and_map_notes(&chart.notes, |n| matches!(n.data, NoteData::Basic(_)));
        let notes_target =
            filter_and_map_notes(&chart.notes, |n| matches!(n.data, NoteData::Target));
        let notes_floor = filter_and_map_notes(&chart.notes, |n| matches!(n.data, NoteData::Floor));

        let notes_evade =
            filter_and_map_notes(&chart.notes, |n| matches!(n.data, NoteData::Evade(_)));
        let notes_contact =
            filter_and_map_notes(&chart.notes, |n| matches!(n.data, NoteData::Contact(_)));

        let notes_basic_hold =
            filter_and_map_notes(&chart.notes, |n| matches!(n.data, NoteData::BasicHold(_)));
        let notes_target_hold =
            filter_and_map_notes(&chart.notes, |n| matches!(n.data, NoteData::TargetHold(_)));
        let notes_floor_hold =
            filter_and_map_notes(&chart.notes, |n| matches!(n.data, NoteData::FloorHold(_)));

        let notes_flick =
            filter_and_map_notes(&chart.notes, |n| matches!(n.data, NoteData::Flick(_)));

        let platform_instances = chart.platforms.iter().map(PlatformInstance::from).collect();
        let platforms_mesh = create_platforms_mesh(&chart.platforms);

        println!("{:#?}", &notes_basic);

        Self {
            notes_basic,
            notes_target,
            notes_floor,

            notes_evade,
            notes_contact,

            notes_basic_hold,
            notes_floor_hold,
            notes_target_hold,

            notes_flick,

            platform_instances,
            platforms_mesh,
        }
    }
}

const PLATFORM_CURVED_SUBDIVISIONS: usize = 36;

fn control_point_to_xz(control_point: BezierControlPoint) -> Vector2<f32> {
    Vector2::new(
        control_point.x_position,
        control_point.time.seconds.unwrap(),
    )
}

fn create_platform_mesh(platform: &Platform) -> Mesh {
    // XXX TODO: Make non curved platforms have less vertices.

    let z0 = platform.start_time.seconds.unwrap();
    let z1 = platform.end_time.seconds.unwrap();

    let v0 = Vector2::new(platform.vertices_x_positions.0, z0);
    let v1 = Vector2::new(platform.vertices_x_positions.1, z1);
    let v2 = Vector2::new(platform.vertices_x_positions.2, z0);
    let v3 = Vector2::new(platform.vertices_x_positions.3, z1);

    let mut control_points_01 = (v0, v1);
    let mut control_points_23 = (v2, v3);

    // XXX TODO: Have a nicer representation of the control points inside `Chart`.
    if !platform.control_points.is_empty() {
        assert!(platform.control_points.len() == 4);

        if platform.control_points[0].is_some() {
            assert!(platform.control_points[1].is_some());
            control_points_01 = (
                control_point_to_xz(platform.control_points[0].unwrap()),
                control_point_to_xz(platform.control_points[1].unwrap()),
            );
        }

        if platform.control_points[2].is_some() {
            assert!(platform.control_points[3].is_some());
            control_points_23 = (
                control_point_to_xz(platform.control_points[2].unwrap()),
                control_point_to_xz(platform.control_points[3].unwrap()),
            );
        }
    }

    Plane::double_sided_cubic_bezier(
        v0,
        v1,
        control_points_01,
        v2,
        v3,
        control_points_23,
        PLATFORM_CURVED_SUBDIVISIONS,
    )
    .into()
}

fn create_platforms_mesh(platforms: &[Platform]) -> Mesh {
    let (vertices, indices) = platforms.iter().fold(
        (Vec::new(), Vec::new()),
        |(mut vertices, mut indices), platform| {
            let mesh = create_platform_mesh(platform);
            vertices.extend(mesh.vertices);
            indices.extend(mesh.indices);

            (vertices, indices)
        },
    );

    Mesh { vertices, indices }
}

fn create_render_description(
    render_objects: &RenderObjects,
    track_settings: &TrackSettings,
) -> RenderDescription {
    let object_instanced_pipeline_description = RenderPipelineDescription {
        rendering_type: RenderingType::Instanced,
        shader_modules: vec![
            ShaderModuleDescriptor {
                source_file_name: String::from("shaders/object_instanced.vs.glsl"),
                shader_stage: ShaderStage::Vertex,
            },
            ShaderModuleDescriptor {
                source_file_name: String::from("shaders/object_instanced.fs.glsl"),
                shader_stage: ShaderStage::Fragment,
            },
        ],
    };

    let object_vertices_instanced_data_pipeline_description = RenderPipelineDescription {
        rendering_type: RenderingType::Instanced,
        shader_modules: vec![
            ShaderModuleDescriptor {
                source_file_name: String::from("shaders/object_vertices_instanced_data.vs.glsl"),
                shader_stage: ShaderStage::Vertex,
            },
            ShaderModuleDescriptor {
                source_file_name: String::from("shaders/object_instanced.fs.glsl"),
                shader_stage: ShaderStage::Fragment,
            },
        ],
    };

    let note_basic_mesh = Polyhedron::cuboid(0.4, 0.1, 0.2);

    let notes_basic_gpu_data =
        note_instances_to_gpu_data_bytes(&render_objects.notes_basic, &track_settings);
    let notes_basic_draw_data = InstancedDrawData {
        rendering_type: RenderingType::Instanced,
        instance_count: notes_basic_gpu_data.len(),
        instance_data: notes_basic_gpu_data,
        vertices: note_basic_mesh.vertices,
        indices: note_basic_mesh.indices,

        pipeline_index: 0,
    };

    // let notes_target_gpu_data =
    //     note_instances_to_gpu_data_bytes(&render_objects.notes_target, &track_settings);
    // let notes_floor_gpu_data =
    //     note_instances_to_gpu_data_bytes(&render_objects.notes_floor, &track_settings);
    // let notes_evade_gpu_data =
    //     note_instances_to_gpu_data_bytes(&render_objects.notes_evade, &track_settings);
    // let notes_contact_gpu_data =
    //     note_instances_to_gpu_data_bytes(&render_objects.notes_contact, &track_settings);

    // let platforms_gpu_data =
    //     platform_instances_to_gpu_data_bytes(&render_objects.platform_instances, &track_settings);
    // let platforms_draw_data = InstancedDrawData {
    //     rendering_type: RenderingType::Instanced,
    //     instance_count: 1,
    //     instance_data: platforms_gpu_data,
    //     vertices: render_objects.platforms_mesh.vertices.clone(),
    //     indices: render_objects.platforms_mesh.indices.clone(),
    //
    //     pipeline_index: 1,
    // };

    RenderDescription {
        pipelines: vec![
            object_instanced_pipeline_description,
            object_vertices_instanced_data_pipeline_description,
        ],
        instanced_draw_data: vec![notes_basic_draw_data],
        scene_uniform_data_size: std::mem::size_of::<SceneUniformGpuData>() as _,
    }
}

pub(crate) struct TrackRenderer {
    renderer: Renderer,
    render_objects: RenderObjects,
    track_settings: TrackSettings,
    scene_uniform: SceneUniformGpuData,
}

impl TrackRenderer {
    pub(crate) fn new(
        window_handle: RawWindowHandle,
        display_handle: RawDisplayHandle,
        track_settings: TrackSettings,
        chart: &Chart,
    ) -> Result<Self> {
        let render_objects = RenderObjects::new_from_chart(chart);
        let render_description = create_render_description(&render_objects, &track_settings);
        let renderer = Renderer::new(window_handle, display_handle, render_description)?;

        Ok(Self {
            renderer,
            render_objects,
            track_settings,
            scene_uniform: SceneUniformGpuData::default(),
        })
    }

    pub fn render(&mut self) -> Result<()> {
        self.renderer
            .update_scene_uniform_data(bytemuck::bytes_of(&self.scene_uniform));
        self.renderer.render()?;

        Ok(())
    }

    pub fn update_view_projection(&mut self, view_projection: Matrix4<f32>) {
        self.scene_uniform.view_projection = view_projection;
    }

    pub fn update_runner_position(&mut self, runner_position: f32) {
        self.scene_uniform.runner_transform =
            Matrix4::new_translation(&Vector3::new(0.0, 0.0, -runner_position));
    }
}
