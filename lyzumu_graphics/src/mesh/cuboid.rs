use nalgebra::Vector3;

use super::Mesh;

pub struct Cuboid {
    vertices: Vec<Vector3<f32>>,
    indices: Vec<u16>,
}

impl Cuboid {
    /// Creates vertices centered at 0,0,0.
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        let x_range = [x / -2.0, x / 2.0];
        let y_range = [y / -2.0, y / 2.0];
        let z_range = [z / -2.0, z / 2.0];

        let vertices = vec![
            // Front face.
            Vector3::new(x_range[0], y_range[0], z_range[0]),
            Vector3::new(x_range[1], y_range[0], z_range[0]),
            Vector3::new(x_range[0], y_range[1], z_range[0]),
            Vector3::new(x_range[1], y_range[1], z_range[0]),
            // Back face.
            Vector3::new(x_range[0], y_range[0], z_range[1]),
            Vector3::new(x_range[1], y_range[0], z_range[1]),
            Vector3::new(x_range[0], y_range[1], z_range[1]),
            Vector3::new(x_range[1], y_range[1], z_range[1]),
        ];
        let indices = vec![
            0, 1, 2, 1, 2, 3, // Front face.
            4, 5, 6, 5, 6, 7, // Back face.
            4, 0, 6, 0, 6, 2, // Left face.
            1, 5, 3, 5, 3, 7, // Right face.
            2, 3, 6, 3, 6, 7, // Top face.
            0, 1, 4, 1, 4, 5, // Bottom face.
        ];

        Self { vertices, indices }
    }
}

impl From<Cuboid> for Mesh {
    fn from(cuboid: Cuboid) -> Self {
        Self {
            vertices: cuboid.vertices,
            indices: cuboid.indices,
        }
    }
}
