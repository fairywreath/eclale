use nalgebra::Vector3;
use std::f32::consts::PI;

use super::Mesh;

#[derive(Clone, Copy)]
pub struct TorusBuilder {
    radius: f32,
    tubular_radius: f32,
    radial_segments: usize,
    tubular_segments: usize,
}

impl TorusBuilder {
    /// # Arguments
    ///
    /// - `radius` is the radius from the center [0, 0, 0] to the center of the tubular radius
    /// - `tubular_radius` is the radius to the surface from the toroidal
    /// - `tubular_segments` is the number of segments that wrap around the tube, it must be at least 3
    /// - `radial_segments` is the number of tube segments requested to generate, it must be at least 3
    pub fn new(
        radius: f32,
        tubular_radius: f32,
        radial_segments: usize,
        tubular_segments: usize,
    ) -> Self {
        assert!(tubular_segments > 2 && radial_segments > 2);
        TorusBuilder {
            radius,
            tubular_radius,
            radial_segments,
            tubular_segments,
        }
    }

    pub fn build_mesh(&self) -> Mesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for j in 0..=self.radial_segments {
            let theta = j as f32 * 2.0 * PI / self.radial_segments as f32;
            let cos_theta = theta.cos();
            let sin_theta = theta.sin();

            for i in 0..=self.tubular_segments {
                let phi = i as f32 * 2.0 * PI / self.tubular_segments as f32;
                let cos_phi = phi.cos();
                let sin_phi = phi.sin();

                let x = (self.radius + self.tubular_radius * cos_phi) * cos_theta;
                let y = self.tubular_radius * sin_phi;
                let z = (self.radius + self.tubular_radius * cos_phi) * sin_theta;

                vertices.push(Vector3::new(x, y, z));
            }
        }

        for j in 1..=self.radial_segments {
            for i in 1..=self.tubular_segments {
                let a = (self.tubular_segments + 1) * j + i - 1;
                let b = (self.tubular_segments + 1) * (j - 1) + i - 1;
                let c = (self.tubular_segments + 1) * (j - 1) + i;
                let d = (self.tubular_segments + 1) * j + i;

                indices.push(a as u16);
                indices.push(b as u16);
                indices.push(d as u16);

                indices.push(b as u16);
                indices.push(c as u16);
                indices.push(d as u16);
            }
        }

        Mesh { vertices, indices }
    }
}
