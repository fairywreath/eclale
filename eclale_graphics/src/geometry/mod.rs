use nalgebra::{Matrix4, Vector3};

pub mod capsule;
pub mod line;
pub mod plane;
pub mod polyhedron;
pub mod sphere;
pub mod torus;

#[derive(Clone)]
pub struct Mesh {
    pub vertices: Vec<Vector3<f32>>,
    pub indices: Vec<u16>,
}

impl Mesh {
    pub fn from_indices(vertices: Vec<Vector3<f32>>, indices: Vec<u16>) -> Self {
        Self { vertices, indices }
    }

    pub fn transform(self: Self, transform: &Matrix4<f32>) -> Self {
        let vertices = self
            .vertices
            .into_iter()
            .map(|v| (transform * v.insert_row(3, 1.0)).xyz())
            .collect::<Vec<_>>();
        Self {
            vertices,
            indices: self.indices,
        }
    }
}
