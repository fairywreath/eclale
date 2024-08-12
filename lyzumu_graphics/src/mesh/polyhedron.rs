use nalgebra::Vector3;

use super::Mesh;

pub struct Polyhedron {
    pub vertices: Vec<Vector3<f32>>,
    pub indices: Vec<u16>,
}

impl Polyhedron {
    /// Cuboid centered at (0,0,0).
    pub fn cuboid(x: f32, y: f32, z: f32) -> Self {
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

    /// Icosahedron centered at (0,0,0).
    pub fn icosahedron(radius: f32) -> Self {
        let t = (1.0 + (5.0 as f32).sqrt()) / 2.0;

        let vertices = vec![
            Vector3::new(-1.0, t, 0.0),
            Vector3::new(1.0, t, 0.0),
            Vector3::new(-1.0, -t, 0.0),
            Vector3::new(1.0, -t, 0.0),
            Vector3::new(0.0, -1.0, t),
            Vector3::new(0.0, 1.0, t),
            Vector3::new(0.0, -1.0, -t),
            Vector3::new(0.0, 1.0, -t),
            Vector3::new(t, 0.0, -1.0),
            Vector3::new(t, 0.0, 1.0),
            Vector3::new(-t, 0.0, -1.0),
            Vector3::new(-t, 0.0, 1.0),
        ]
        .into_iter()
        .map(|v| v.normalize() * radius)
        .collect();

        let indices = vec![
            0, 11, 5, 0, 5, 1, 0, 1, 7, 0, 7, 10, 0, 10, 11, 1, 5, 9, 5, 11, 4, 11, 10, 2, 10, 7,
            6, 7, 1, 8, 3, 9, 4, 3, 4, 2, 3, 2, 6, 3, 6, 8, 3, 8, 9, 4, 9, 5, 2, 4, 11, 6, 2, 10,
            8, 6, 7, 9, 8, 1,
        ];

        Self { vertices, indices }
    }

    /// Octahedron with its "pyramid base" in the XY axes.
    /// Length and width starts at 0 in the x and z axes respectively.
    pub fn octahedron(width: f32, length: f32) -> Self {
        let x_range = (width / -2.0, width / 2.0);
        let y_range = (0.0, -width);
        let z_range = (0.0, length);

        let x_center = (x_range.0 + x_range.1) / 2.0;
        let y_center = (y_range.0 + y_range.1) / 2.0;
        let z_center = (z_range.0 + z_range.1) / 2.0;

        let vertices = vec![
            // First tip vertex.
            Vector3::new(x_center, y_center, z_range.0),
            // "Pyramid base" vertices.
            Vector3::new(x_center, y_range.0, z_center),
            Vector3::new(x_range.0, y_center, z_center),
            Vector3::new(x_center, y_range.1, z_center),
            Vector3::new(x_range.1, y_center, z_center),
            // Second tip vertex.
            Vector3::new(x_center, y_center, z_range.1),
        ];

        #[rustfmt::skip]
        let indices = vec![
            // First pyramid.
            0, 1, 2,
            0, 2, 3,
            0, 3, 4,
            0, 4, 1,
            // Second pyramid.
            5, 1, 2,
            5, 2, 3,
            5, 3, 4,
            5, 4, 1,
        ];

        Self { vertices, indices }
    }
}

impl From<Polyhedron> for Mesh {
    fn from(cuboid: Polyhedron) -> Self {
        Self {
            vertices: cuboid.vertices,
            indices: cuboid.indices,
        }
    }
}
