use std::collections::HashMap;

use eclale_chart::{
    Chart, ChartData, ContactNote, EvadeNote, FlickNote, HitNote, HitNoteType, HoldNote, Lane,
    LaneType, TrackPosition,
};
use nalgebra::{Matrix4, Vector2, Vector3, Vector4};

use eclale_graphics::geometry::{plane::Plane, Mesh};

use super::track_renderer::HIT_X_LENGTH;

#[derive(Clone, Debug)]
pub(crate) struct TrackSettings {
    pub(crate) runner_speed: f32,
}

#[derive(Clone, Debug)]
pub(crate) struct NoteInstance {
    pub(crate) z_position: f32,
    pub(crate) x_position: f32,
    pub(crate) base_color: Vector4<f32>,

    pub(crate) apply_runner_transform: bool,
}

fn get_base_color_hit(hit_type: HitNoteType) -> Vector4<f32> {
    match hit_type {
        // XXX TODO: Have proper type enums.
        HitNoteType(0) | HitNoteType(10) => Vector4::new(1.0, 0.0, 1.0, 1.0),
        HitNoteType(1) | HitNoteType(11) => Vector4::new(0.5, 0.0, 0.5, 1.0),
        HitNoteType(2) | HitNoteType(12) => Vector4::new(1.0, 0.0, 0.0, 1.0),
        HitNoteType(3) | HitNoteType(13) => Vector4::new(0.0, 1.0, 0.0, 1.0),
        HitNoteType(4) | HitNoteType(14) => Vector4::new(0.0, 0.0, 1.0, 1.0),
        _ => {
            log::warn!("Encountered unknown hit note type {:?}", hit_type);
            Vector4::new(1.0, 1.0, 1.0, 1.0)
        }
    }
}

fn get_base_color_lane(lane_type: LaneType) -> Vector4<f32> {
    match lane_type {
        // XXX TODO: Have proper type enums.
        LaneType(1) | LaneType(12) => Vector4::new(1.0, 0.0, 0.0, 1.0),
        LaneType(2) | LaneType(13) => Vector4::new(0.0, 1.0, 0.0, 1.0),
        LaneType(3) | LaneType(14) => Vector4::new(0.0, 0.0, 1.0, 1.0),
        _ => {
            log::warn!("Encountered unknown lane note type {:?}", lane_type);
            Vector4::new(1.0, 1.0, 1.0, 1.0)
        }
    }
}

impl NoteInstance {
    fn from_chart_hit(note: &HitNote, apply_runner_transform: bool) -> Self {
        Self {
            z_position: note.position.z,
            x_position: note.position.x,
            base_color: get_base_color_hit(note.ty),
            apply_runner_transform,
        }
    }

    fn from_chart_contact(note: &ContactNote, apply_runner_transform: bool) -> Self {
        Self {
            z_position: note.position.z,
            x_position: note.position.x,
            base_color: Vector4::new(1.0, 1.0, 0.0, 1.0),
            apply_runner_transform,
        }
    }

    fn from_chart_flick(note: &FlickNote, apply_runner_transform: bool) -> Self {
        Self {
            z_position: note.position.z,
            x_position: note.position.x,
            base_color: Vector4::new(0.8, 0.8, 0.1, 1.0),
            apply_runner_transform,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PlatformInstance {
    pub(crate) z_start_position: f32,
    pub(crate) z_end_position: f32,
    pub(crate) base_color: Vector4<f32>,
}

#[derive(Clone)]
pub(crate) struct HoldNotesDescription {
    // XXX TODO:  Use a hold note object type.
    pub(crate) objects: Vec<PlatformInstance>,
    pub(crate) mesh: Mesh,
    /// Index to the object array for each vertex in the mesh.
    pub(crate) objects_indices: Vec<usize>,
}

#[derive(Clone)]
pub(crate) struct LanesDescription {
    // XXX TODO:  Use a hold note object type.
    pub(crate) objects: Vec<PlatformInstance>,
    pub(crate) mesh: Mesh,
    /// Index to the object array for each vertex in the mesh.
    pub(crate) objects_indices: Vec<usize>,
}

#[derive(Clone, Debug)]
pub(crate) struct EvadeNoteInstance {
    /// Start position in the XZ grid.
    pub(crate) start_position: Vector2<f32>,

    /// End position in the XZ grid.
    pub(crate) end_position: Vector2<f32>,

    pub(crate) base_color: Vector4<f32>,

    // Time to start note movement in seconds.
    pub(crate) trigger_time: f32,

    // Duration of movement in seconds.
    pub(crate) duration: f32,
}

impl EvadeNoteInstance {
    fn from_chart_evade(note: &EvadeNote) -> Self {
        Self {
            start_position: Vector2::new(note.movement.start.x, note.movement.start.z),
            end_position: Vector2::new(note.movement.end.x, note.movement.end.z),
            base_color: Vector4::new(0.6, 0.2, 0.9, 1.0),
            trigger_time: note.movement.trigger_time.0,
            duration: note.movement.duration,
        }
    }
}

#[derive(Clone)]
pub(crate) struct TrackDescription {
    pub(crate) notes_hit: Vec<NoteInstance>,
    pub(crate) notes_contact: Vec<NoteInstance>,
    pub(crate) notes_flick: Vec<NoteInstance>,

    pub(crate) notes_evade: Vec<EvadeNoteInstance>,

    pub(crate) settings: TrackSettings,

    pub(crate) platform_instances: Vec<PlatformInstance>,
    pub(crate) platform_mesh: Mesh,

    pub(crate) hold_notes: HoldNotesDescription,

    pub(crate) lanes: LanesDescription,
}

struct TrackDescriptionCreator {
    settings: TrackSettings,
}

impl TrackDescriptionCreator {
    fn new(settings: TrackSettings) -> Self {
        Self { settings }
    }

    fn apply_runner_speed(&self, notes: Vec<NoteInstance>) -> Vec<NoteInstance> {
        notes
            .into_iter()
            .map(|mut n| {
                n.z_position = n.z_position * self.settings.runner_speed;
                n
            })
            .collect()
    }

    fn apply_runner_speed_on_evade_notes(
        &self,
        notes: Vec<EvadeNoteInstance>,
    ) -> Vec<EvadeNoteInstance> {
        notes
            .into_iter()
            .map(|mut n| {
                // y is z here as Vector2 is used to represent position in the zx plane..
                n.start_position.y = n.start_position.y * self.settings.runner_speed;
                n.end_position.y = n.end_position.y * self.settings.runner_speed;
                n
            })
            .collect()
    }

    fn track_position_to_xz_vertices(
        &self,
        track_positions: &[TrackPosition],
    ) -> Vec<Vector3<f32>> {
        track_positions
            .iter()
            .map(|p| Vector3::new(p.x, 0.0, p.z * self.settings.runner_speed))
            .collect()
    }

    fn create_platform_mesh(&self, platform: &eclale_chart::Platform) -> Mesh {
        let walls_left_vertices = self.track_position_to_xz_vertices(&platform.points_left);
        let walls_right_vertices = self.track_position_to_xz_vertices(&platform.points_right);

        println!(
            "Walls vertices len {} {}",
            walls_left_vertices.len(),
            walls_right_vertices.len()
        );

        let mesh =
            Plane::triangulate_from_two_sides(walls_left_vertices, walls_right_vertices).to_mesh();

        println!(
            "Platform mesh vert {} indices {}",
            mesh.vertices.len(),
            mesh.indices.len()
        );

        mesh
    }

    fn create_plane_mesh_from_points(&self, points: &[TrackPosition], width: f32) -> Mesh {
        let left_points = self
            .track_position_to_xz_vertices(points)
            .into_iter()
            .map(|mut point| {
                point.x = point.x - (width / 2.0);
                point
            })
            .collect();
        let right_points = self
            .track_position_to_xz_vertices(points)
            .into_iter()
            .map(|mut point| {
                point.x = point.x + (width / 2.0);
                point
            })
            .collect();
        Plane::triangulate_from_two_sides(left_points, right_points).to_mesh()
    }

    fn create_lanes(&self, lanes: &HashMap<LaneType, Vec<Lane>>) -> LanesDescription {
        let (vertices, indices, objects, objects_indices) = lanes.iter().fold(
            (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
            |(mut vertices, mut indices, mut objects, mut objects_indices), (lane_type, lanes)| {
                for lane in lanes {
                    // If not enemy lane. XXX TODO: Have proper enum type.
                    if *lane_type != LaneType(4) {
                        let mesh = self.create_plane_mesh_from_points(&lane.points, 0.04);

                        // Append object index to global object indices array.
                        let current_object_index = objects.len();
                        objects_indices.extend(
                            std::iter::repeat(current_object_index).take(mesh.vertices.len()),
                        );

                        // Append to global vertex indices array.
                        let current_index_offset = vertices.len() as u16;
                        vertices.extend(mesh.vertices);

                        for index in mesh.indices {
                            indices.push(index + current_index_offset);
                        }

                        objects.push(PlatformInstance {
                            base_color: get_base_color_lane(*lane_type),
                            // XXX TODO: Fill these properly.
                            z_start_position: 0.0,
                            z_end_position: 0.0,
                        });
                    }
                }
                (vertices, indices, objects, objects_indices)
            },
        );

        assert_eq!(vertices.len(), objects_indices.len());
        assert_eq!(*objects_indices.last().unwrap(), objects.len() - 1);

        let mesh = Mesh { vertices, indices };
        let mesh = mesh.transform(&Matrix4::new_translation(&Vector3::new(0.0, -0.005, 0.0)));

        // println!(
        //     "Lane notes mesh vert {} indices {}",
        //     mesh.vertices.len(),
        //     mesh.indices.len()
        // );

        LanesDescription {
            mesh,
            objects,
            objects_indices,
        }
    }

    fn create_hold_note_mesh(&self, hold_note: &HoldNote) -> Mesh {
        self.create_plane_mesh_from_points(&hold_note.points, HIT_X_LENGTH)
    }

    fn create_hold_notes(&self, hold_notes: &[HoldNote]) -> HoldNotesDescription {
        let (vertices, indices, objects, objects_indices) = hold_notes.iter().fold(
            (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
            |(mut vertices, mut indices, mut objects, mut objects_indices), note| {
                let mesh = self.create_hold_note_mesh(&note);

                // Append object index to global object indices array.
                let current_object_index = objects.len();
                objects_indices
                    .extend(std::iter::repeat(current_object_index).take(mesh.vertices.len()));

                // Append to global vertex indices array.
                let current_index_offset = vertices.len() as u16;
                vertices.extend(mesh.vertices);

                for index in mesh.indices {
                    indices.push(index + current_index_offset);
                }

                objects.push(PlatformInstance {
                    base_color: get_base_color_hit(note.ty),
                    // XXX TODO: Fill these properly.
                    z_start_position: 0.0,
                    z_end_position: 0.0,
                });

                (vertices, indices, objects, objects_indices)
            },
        );

        assert_eq!(vertices.len(), objects_indices.len());
        assert_eq!(*objects_indices.last().unwrap(), objects.len() - 1);

        let mesh = Mesh { vertices, indices };
        let mesh = mesh.transform(&Matrix4::new_translation(&Vector3::new(0.0, -0.005, 0.0)));

        // println!(
        //     "Hold notes mesh vert {} indices {}",
        //     mesh.vertices.len(),
        //     mesh.indices.len()
        // );

        HoldNotesDescription {
            mesh,
            objects,
            objects_indices,
        }
    }

    // Created positions are affected by the runner speed.
    fn create(self, chart: &ChartData) -> TrackDescription {
        let notes_hit = chart
            .notes
            .hits
            .iter()
            .map(|n| NoteInstance::from_chart_hit(n, true))
            .collect();
        let notes_contact = chart
            .notes
            .contacts
            .iter()
            .map(|n| NoteInstance::from_chart_contact(n, true))
            .collect();
        let notes_flick = chart
            .notes
            .flicks
            .iter()
            .map(|n| NoteInstance::from_chart_flick(n, true))
            .collect();
        let notes_evade = chart
            .notes
            .evades
            .iter()
            .map(|n| EvadeNoteInstance::from_chart_evade(n))
            .collect();

        let notes_hit = self.apply_runner_speed(notes_hit);
        let notes_contact = self.apply_runner_speed(notes_contact);
        let notes_flick = self.apply_runner_speed(notes_flick);
        let notes_evade = self.apply_runner_speed_on_evade_notes(notes_evade);

        // XXX TODO: Handle case where more than one platform instance exists.
        let platform_mesh = self.create_platform_mesh(&chart.track.platforms[0]);

        let hold_notes = self.create_hold_notes(&chart.notes.holds);

        let lanes = self.create_lanes(&chart.track.lanes);

        TrackDescription {
            notes_hit,
            notes_contact,
            notes_flick,

            notes_evade,

            settings: self.settings,

            platform_mesh,
            // XXX TODO: Properly fill this.
            platform_instances: vec![PlatformInstance {
                z_start_position: 0.0,
                z_end_position: 200.0,
                base_color: Vector4::new(0.0, 0.0, 0.0, 1.0),
            }],

            hold_notes,
            lanes,
        }
    }
}

impl TrackDescription {
    pub(crate) fn from_chart(chart: &Chart, settings: TrackSettings) -> Self {
        TrackDescriptionCreator::new(settings).create(&chart.data)
    }
}
