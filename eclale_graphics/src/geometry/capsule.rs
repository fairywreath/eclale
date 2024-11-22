use nalgebra::Vector2;
use nalgebra::Vector3;
use std::f32::consts::PI;

use super::Mesh;

/// UV profile for the capsule mesh.
#[derive(Clone, Copy, Debug, Default)]
pub enum CapsuleUvProfile {
    #[default]
    Aspect,
    Uniform,
    Fixed,
}

#[derive(Clone, Copy, Debug)]
pub struct CapsuleBuilder {
    pub radius: f32,
    pub height: f32,
    pub rings: u32,
    pub longitudes: u32,
    pub latitudes: u32,
    pub uv_profile: CapsuleUvProfile,
}

impl Default for CapsuleBuilder {
    fn default() -> Self {
        Self {
            radius: 1.0,
            height: 1.0,
            rings: 0,
            longitudes: 32,
            latitudes: 16,
            uv_profile: CapsuleUvProfile::default(),
        }
    }
}

impl CapsuleBuilder {
    pub fn new(radius: f32, height: f32, longitudes: u32, latitudes: u32) -> Self {
        Self {
            radius,
            height,
            longitudes,
            latitudes,
            ..Default::default()
        }
    }

    pub const fn rings(mut self, rings: u32) -> Self {
        self.rings = rings;
        self
    }

    pub const fn longitudes(mut self, longitudes: u32) -> Self {
        self.longitudes = longitudes;
        self
    }

    pub const fn latitudes(mut self, latitudes: u32) -> Self {
        self.latitudes = latitudes;
        self
    }

    pub const fn uv_profile(mut self, uv_profile: CapsuleUvProfile) -> Self {
        self.uv_profile = uv_profile;
        self
    }

    pub fn build_mesh(&self) -> Mesh {
        let radius = self.radius;
        let half_length = self.height / 2.0;
        let rings = self.rings;
        let longitudes = self.longitudes;
        let latitudes = self.latitudes;
        let uv_profile = self.uv_profile;

        let calc_middle = rings > 0;
        let half_lats = latitudes / 2;
        let half_latsn1 = half_lats - 1;
        let half_latsn2 = half_lats - 2;
        let ringsp1 = rings + 1;
        let lonsp1 = longitudes + 1;
        let summit = half_length + radius;

        // Vertex index offsets.
        let vert_offset_north_hemi = longitudes;
        let vert_offset_north_equator = vert_offset_north_hemi + lonsp1 * half_latsn1;
        let vert_offset_cylinder = vert_offset_north_equator + lonsp1;
        let vert_offset_south_equator = if calc_middle {
            vert_offset_cylinder + lonsp1 * rings
        } else {
            vert_offset_cylinder
        };
        let vert_offset_south_hemi = vert_offset_south_equator + lonsp1;
        let vert_offset_south_polar = vert_offset_south_hemi + lonsp1 * half_latsn2;
        let vert_offset_south_cap = vert_offset_south_polar + lonsp1;

        // Initialize arrays.
        let vert_len = (vert_offset_south_cap + longitudes) as usize;

        let mut vertices: Vec<Vector3<f32>> = vec![Vector3::zeros(); vert_len];
        let mut uvs: Vec<Vector2<f32>> = vec![Vector2::zeros(); vert_len];
        let mut normals: Vec<Vector3<f32>> = vec![Vector3::zeros(); vert_len];

        let to_theta = 2.0 * PI / longitudes as f32;
        let to_phi = PI / latitudes as f32;
        let to_tex_horizontal = 1.0 / longitudes as f32;
        let to_tex_vertical = 1.0 / half_lats as f32;

        let vt_aspect_ratio = match uv_profile {
            CapsuleUvProfile::Aspect => radius / (2.0 * half_length + radius + radius),
            CapsuleUvProfile::Uniform => half_lats as f32 / (ringsp1 + latitudes) as f32,
            CapsuleUvProfile::Fixed => 1.0 / 3.0,
        };
        let vt_aspect_north = 1.0 - vt_aspect_ratio;
        let vt_aspect_south = vt_aspect_ratio;

        let mut theta_cartesian: Vec<Vector2<f32>> = vec![Vector2::zeros(); longitudes as usize];
        let mut rho_theta_cartesian: Vec<Vector2<f32>> =
            vec![Vector2::zeros(); longitudes as usize];
        let mut s_texture_cache: Vec<f32> = vec![0.0; lonsp1 as usize];

        for j in 0..longitudes as usize {
            let jf = j as f32;
            let s_texture_polar = 1.0 - ((jf + 0.5) * to_tex_horizontal);
            let theta = jf * to_theta;

            let cos_theta = theta.cos();
            let sin_theta = theta.sin();

            theta_cartesian[j] = Vector2::new(cos_theta, sin_theta);
            rho_theta_cartesian[j] = Vector2::new(radius * cos_theta, radius * sin_theta);

            // North.
            vertices[j] = Vector3::new(0.0, summit, 0.0);
            uvs[j] = Vector2::new(s_texture_polar, 1.0);
            normals[j] = Vector3::new(0.0, 1.0, 0.0);

            // South.
            let idx = vert_offset_south_cap as usize + j;
            vertices[idx] = Vector3::new(0.0, -summit, 0.0);
            uvs[idx] = Vector2::new(s_texture_polar, 0.0);
            normals[idx] = Vector3::new(0.0, -1.0, 0.0);
        }

        // Equatorial vertices.
        for (j, s_texture_cache_j) in s_texture_cache.iter_mut().enumerate().take(lonsp1 as usize) {
            let s_texture = 1.0 - j as f32 * to_tex_horizontal;
            *s_texture_cache_j = s_texture;

            let j_mod = j % longitudes as usize;
            let tc = theta_cartesian[j_mod];
            let rtc = rho_theta_cartesian[j_mod];

            // North equator.
            let idxn = vert_offset_north_equator as usize + j;
            vertices[idxn] = Vector3::new(rtc.x, half_length, -rtc.y);
            uvs[idxn] = Vector2::new(s_texture, vt_aspect_north);
            normals[idxn] = Vector3::new(tc.x, 0.0, -tc.y);

            // South equator.
            let idxs = vert_offset_south_equator as usize + j;
            vertices[idxs] = Vector3::new(rtc.x, -half_length, -rtc.y);
            uvs[idxs] = Vector2::new(s_texture, vt_aspect_south);
            normals[idxs] = Vector3::new(tc.x, 0.0, -tc.y);
        }

        // Hemisphere vertices.
        for i in 0..half_latsn1 {
            let ip1f = i as f32 + 1.0;
            let phi = ip1f * to_phi;

            let cos_phi_south = phi.cos();
            let sin_phi_south = phi.sin();

            let cos_phi_north = sin_phi_south;
            let sin_phi_north = -cos_phi_south;

            let rho_cos_phi_north = radius * cos_phi_north;
            let rho_sin_phi_north = radius * sin_phi_north;
            let z_offset_north = half_length - rho_sin_phi_north;

            let rho_cos_phi_south = radius * cos_phi_south;
            let rho_sin_phi_south = radius * sin_phi_south;
            let z_offset_sout = -half_length - rho_sin_phi_south;

            let t_tex_fac = ip1f * to_tex_vertical;
            let cmpl_tex_fac = 1.0 - t_tex_fac;
            let t_tex_north = cmpl_tex_fac + vt_aspect_north * t_tex_fac;
            let t_tex_south = cmpl_tex_fac * vt_aspect_south;

            let i_lonsp1 = i * lonsp1;
            let vert_curr_lat_north = vert_offset_north_hemi + i_lonsp1;
            let vert_curr_lat_south = vert_offset_south_hemi + i_lonsp1;

            for (j, s_texture) in s_texture_cache.iter().enumerate().take(lonsp1 as usize) {
                let j_mod = j % longitudes as usize;

                let tc = theta_cartesian[j_mod];

                // North hemisphere.
                let idxn = vert_curr_lat_north as usize + j;
                vertices[idxn] = Vector3::new(
                    rho_cos_phi_north * tc.x,
                    z_offset_north,
                    -rho_cos_phi_north * tc.y,
                );
                uvs[idxn] = Vector2::new(*s_texture, t_tex_north);
                normals[idxn] =
                    Vector3::new(tc.x * cos_phi_north, sin_phi_north, -tc.y * cos_phi_north);

                // South hemisphere.
                let idxs = vert_curr_lat_south as usize + j;
                vertices[idxs] = Vector3::new(
                    rho_cos_phi_south * tc.x,
                    z_offset_sout,
                    -rho_cos_phi_south * tc.y,
                );
                uvs[idxs] = Vector2::new(*s_texture, t_tex_south);
                normals[idxs] =
                    Vector3::new(tc.x * cos_phi_south, -sin_phi_south, -tc.y * cos_phi_south);
            }
        }

        // let indices = (0..vert_len as u16).collect::<Vec<_>>();
        let mut indices = Vec::new();
        // Build the indices for the north cap
        for i in 0..longitudes {
            let next_i = (i + 1) % longitudes;
            indices.push(i * 2);
            indices.push(next_i * 2);
            indices.push(longitudes * 2); // Center vertex of the cap
        }

        // Build the indices for the south cap
        let base = (longitudes + 1) * 2;
        for i in 0..longitudes {
            let next_i = (i + 1) % longitudes;
            indices.push(base + (i * 2 + 1));
            indices.push(base + (next_i * 2 + 1));
            indices.push(base + 1); // Center vertex of the cap
        }

        // Build the indices for the cylindrical part
        for j in 0..latitudes {
            let row_start = (j * (longitudes + 1)) + (longitudes + 1) * 2;
            let next_row_start = row_start + (longitudes + 1);

            for i in 0..longitudes {
                let next_i = (i + 1) % longitudes;

                indices.push(row_start + i);
                indices.push(next_row_start + i);
                indices.push(next_row_start + next_i);

                indices.push(row_start + i);
                indices.push(next_row_start + next_i);
                indices.push(row_start + next_i);
            }
        }

        let indices = indices.into_iter().map(|i| i as u16).collect::<Vec<_>>();

        Mesh { vertices, indices }
    }
}
