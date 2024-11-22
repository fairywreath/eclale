use std::f32::consts::PI;

use nalgebra::Vector3;

use super::Mesh;

/// Represents a sphere centered at (0, 0, 0).
pub struct SphereBuilder {
    radius: f32,
    sub_u: usize,
    sub_v: usize,
}

impl SphereBuilder {
    /// Creates a new sphere.
    ///
    /// # Arguments
    ///
    /// - `radius` is the radius of the sphere.
    /// - `u` is the number of points across the equator of the sphere, must be at least 2.
    /// - `v` is the number of points from pole to pole, must be at least 2.
    ///
    /// # Panics
    ///
    /// This function panics if `u` or `v` are less than 2 respectively.
    pub fn new(radius: f32, u: usize, v: usize) -> Self {
        assert!(u > 1 && v > 1);
        Self {
            radius,
            sub_u: u,
            sub_v: v,
        }
    }

    fn vert(&self, u: usize, v: usize) -> Vector3<f32> {
        let u = (u as f32 / self.sub_u as f32) * PI * 2.;
        let v = (v as f32 / self.sub_v as f32) * PI;

        Vector3::new(
            self.radius * u.cos() * v.sin(),
            self.radius * u.sin() * v.sin(),
            self.radius * v.cos(),
        )
    }

    pub fn build_mesh(&self) -> Mesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Generate vertices
        for v in 0..=self.sub_v {
            for u in 0..self.sub_u {
                vertices.push(self.vert(u, v));
            }
        }

        // Generate indices
        for v in 0..self.sub_v {
            for u in 0..self.sub_u {
                let u1 = (u + 1) % self.sub_u;

                let i0 = (v * self.sub_u + u) as u16;
                let i1 = ((v + 1) * self.sub_u + u) as u16;
                let i2 = ((v + 1) * self.sub_u + u1) as u16;
                let i3 = (v * self.sub_u + u1) as u16;

                if v == 0 {
                    indices.push(i0);
                    indices.push(i1);
                    indices.push(i2);
                } else if v == self.sub_v - 1 {
                    indices.push(i0);
                    indices.push(i2);
                    indices.push(i3);
                } else {
                    indices.push(i0);
                    indices.push(i1);
                    indices.push(i2);

                    indices.push(i0);
                    indices.push(i2);
                    indices.push(i3);
                }
            }
        }

        Mesh { vertices, indices }
    }
}
