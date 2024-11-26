use nalgebra::{Vector2, Vector3};

use super::{line, Mesh};

pub struct Plane {
    pub vertices: Vec<Vector3<f32>>,
    pub indices: Vec<u16>,
}

/// Transforms a 2D coordinate to 3D on the xz axis.
fn to_xz(v: Vector2<f32>) -> Vector3<f32> {
    Vector3::new(v.x, 0.0, v.y)
}

impl Plane {
    /// Plane on the xz axis.
    pub fn quad(v0: Vector2<f32>, v1: Vector2<f32>, v2: Vector2<f32>, v3: Vector2<f32>) -> Self {
        Self {
            vertices: vec![to_xz(v0), to_xz(v1), to_xz(v2), to_xz(v3)],
            indices: vec![0, 1, 2, 1, 2, 3],
        }
    }

    /// Plane on the xz axis where one side, denoted by v0 and v1, has a bezier curve.
    pub fn single_sided_cubic_bezier(
        v0: Vector2<f32>,
        v1: Vector2<f32>,
        control_points: (Vector2<f32>, Vector2<f32>),
        v2: Vector2<f32>,
        v3: Vector2<f32>,
        subdivions: usize,
    ) -> Self {
        Self::triangulate_from_two_sides(
            line::cubic_bezier_curve_points_xz(v0, v1, control_points, subdivions),
            vec![to_xz(v2), to_xz(v3)],
        )
    }

    pub fn double_sided_cubic_bezier(
        v0: Vector2<f32>,
        v1: Vector2<f32>,
        control_points_01: (Vector2<f32>, Vector2<f32>),
        v2: Vector2<f32>,
        v3: Vector2<f32>,
        control_points_23: (Vector2<f32>, Vector2<f32>),
        subdivisions: usize,
    ) -> Self {
        Self::triangulate_from_two_sides(
            line::cubic_bezier_curve_points_xz(v0, v1, control_points_01, subdivisions),
            line::cubic_bezier_curve_points_xz(v2, v3, control_points_23, subdivisions),
        )
    }

    /// Plane on the zx-axis where edges on both z axis sides are parallel cubic bezier curves.
    /// `width` denotes the x axis range.
    pub fn double_sided_parallel_cubic_bezier(
        v0: Vector2<f32>,
        v1: Vector2<f32>,
        control_points: (Vector2<f32>, Vector2<f32>),
        width: f32,
        subdivisions: usize,
    ) -> Self {
        let curve_points_0 =
            line::cubic_bezier_curve_points_xz(v0, v1, control_points, subdivisions);
        let curve_points_1 = curve_points_0
            .iter()
            .map(|p| Vector3::new(p.x + width, p.y, p.z))
            .collect::<Vec<_>>();

        Self::triangulate_from_two_sides(curve_points_0, curve_points_1)
    }

    /// Creates a set of vertices and indices, composed of triangles, from two sides(set of unique vertices)
    /// that are (assumed to be) in the same axis to create a proper rplane.
    pub fn triangulate_from_two_sides(
        mut side_a_vertices: Vec<Vector3<f32>>,
        side_b_vertices: Vec<Vector3<f32>>,
    ) -> Self {
        let side_a_count = side_a_vertices.len() as u16;
        let side_b_count = side_b_vertices.len() as u16;
        assert!(side_a_count >= 2);
        assert!(side_b_count >= 2);

        // let vertices = {
        //     side_a_vertices.extend(side_b_vertices);
        //     side_a_vertices
        // };

        // XXX: Make this a separate function.
        // We duplicate some vertices here so gl_VertexIndex can signify whether a vertex is
        // at the left side or right side.
        let mut vertices = Vec::with_capacity(side_a_vertices.len() + side_b_vertices.len());
        let mut a_iter = side_a_vertices.into_iter();
        let mut b_iter = side_b_vertices.into_iter();
        let mut last_a_index = 0;
        let mut last_b_index = 0;

        for (a, b) in a_iter.by_ref().zip(b_iter.by_ref()) {
            last_a_index = vertices.len();
            vertices.push(a);
            last_b_index = vertices.len();
            vertices.push(b);
        }
        for a in a_iter {
            vertices.push(a);
            vertices.push(vertices[last_b_index]);
        }
        for b in b_iter {
            vertices.push(vertices[last_a_index]);
            vertices.push(b);
        }

        assert!(vertices.len() >= 4);
        assert!(vertices.len() % 2 == 0);

        let mut indices = Vec::new();
        for i in (0..vertices.len() as u16 - 2).step_by(2) {
            indices.push(i);
            indices.push(i + 1);
            indices.push(i + 2);

            indices.push(i + 1);
            indices.push(i + 2);
            indices.push(i + 3);
        }

        // let mut current_side_a_offset = 0 as u16;
        // let mut current_side_b_offset = side_a_count;
        // for i in 1..side_a_count {
        //     // Side b's last vertex is reached.
        //     if i >= side_b_count {
        //         break;
        //     }
        //
        //     indices.push((i - 1) as _);
        //     indices.push(i);
        //     indices.push(i - 1 + side_a_count);
        //
        //     indices.push(i - 1 + side_a_count);
        //     indices.push(i);
        //     indices.push(i + side_a_count);
        //
        //     current_side_a_offset = i;
        //     current_side_b_offset = i + side_a_count;
        // }
        //
        // // If we have remaining side a vertices to add, make triangles with the last vertex of side b.
        // for i in current_side_a_offset + 1..side_a_count {
        //     indices.push(i - 1);
        //     indices.push(i);
        //     indices.push(side_a_count + side_b_count - 1);
        // }
        //
        // // If we have remaining side b vertices to add, make triangles with the last vertex of side a.
        // for i in current_side_b_offset + 1..side_a_count + side_b_count {
        //     indices.push(i);
        //     indices.push(i - 1);
        //     indices.push(side_a_count - 1);
        // }

        Self { vertices, indices }
    }

    pub fn to_mesh(self) -> Mesh {
        Mesh {
            vertices: self.vertices,
            indices: self.indices,
        }
    }
}
