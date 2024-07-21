use nalgebra::{Matrix4, Vector4};

use crate::mesh::Mesh;

///
/// Describes how the game/track scene.
///

#[derive(Clone)]
pub struct SceneHitObject {
    pub transform: Matrix4<f32>,
    pub base_color: Vector4<f32>,
}

pub struct TrackScene {
    pub hit_objects: Vec<SceneHitObject>,
    pub hit_object_mesh: Mesh,
}
