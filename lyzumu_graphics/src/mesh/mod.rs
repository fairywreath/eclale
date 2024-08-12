use nalgebra::Vector3;

pub mod capsule;
pub mod plane;
pub mod polyhedron;
pub mod sphere;
pub mod torus;

pub struct Mesh {
    pub vertices: Vec<Vector3<f32>>,
    pub indices: Vec<u16>,
}
