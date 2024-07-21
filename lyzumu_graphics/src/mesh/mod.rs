use nalgebra::Vector3;

pub mod cuboid;
pub mod plane;

pub struct Mesh {
    pub vertices: Vec<Vector3<f32>>,
    pub indices: Vec<u16>,
}
